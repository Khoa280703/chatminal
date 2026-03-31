use std::time::{Duration, Instant};

use super::{RuntimeState, trim_live_output};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartupRecipeStep {
    RunLine(String),
    TypeText(String),
    Enter,
    SleepMs(u64),
    WaitFor {
        needle: String,
        timeout_ms: u64,
    },
}

const DEFAULT_WAIT_TIMEOUT_MS: u64 = 10_000;
const STARTUP_RECIPE_WAIT_POLL_MS: u64 = 50;
const RECENT_OUTPUT_TAIL_MAX_BYTES: usize = 16 * 1024;

pub fn parse_startup_recipe(raw: &str) -> Result<Vec<StartupRecipeStep>, String> {
    let mut steps = Vec::new();
    for segment in raw
        .split(['\n', ';'])
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
    {
        if segment.starts_with('#') {
            continue;
        }
        if let Some(rest) = segment.strip_prefix("run ") {
            steps.push(StartupRecipeStep::RunLine(rest.to_string()));
            continue;
        }
        if let Some(rest) = segment.strip_prefix("type ") {
            steps.push(StartupRecipeStep::TypeText(rest.to_string()));
            continue;
        }
        if segment.eq_ignore_ascii_case("enter") {
            steps.push(StartupRecipeStep::Enter);
            continue;
        }
        if let Some(rest) = segment
            .strip_prefix("sleep ")
            .or_else(|| segment.strip_prefix("wait "))
        {
            let ms = rest
                .trim()
                .parse::<u64>()
                .map_err(|_| format!("startup recipe sleep/wait invalid: {segment}"))?;
            steps.push(StartupRecipeStep::SleepMs(ms));
            continue;
        }
        if let Some(rest) = segment.strip_prefix("wait-for ") {
            let (needle, timeout_ms) = parse_wait_for(rest)?;
            steps.push(StartupRecipeStep::WaitFor { needle, timeout_ms });
            continue;
        }
        steps.push(StartupRecipeStep::RunLine(segment.to_string()));
    }
    Ok(steps)
}

fn parse_wait_for(raw: &str) -> Result<(String, u64), String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("startup recipe wait-for missing needle".to_string());
    }

    if let Some((needle, timeout)) = trimmed.rsplit_once(" timeout=") {
        let timeout_ms = timeout
            .trim()
            .parse::<u64>()
            .map_err(|_| format!("startup recipe wait-for timeout invalid: {raw}"))?;
        let needle = needle.trim();
        if needle.is_empty() {
            return Err("startup recipe wait-for missing needle".to_string());
        }
        return Ok((needle.to_string(), timeout_ms));
    }

    Ok((trimmed.to_string(), DEFAULT_WAIT_TIMEOUT_MS))
}

impl RuntimeState {
    pub(super) fn spawn_startup_recipe_runner(&self, session_id: &str) -> Result<(), String> {
        let (generation, steps) = self.prepare_startup_recipe_run(session_id)?;
        if steps.is_empty() {
            return Ok(());
        }

        let state = self.clone();
        let session_id = session_id.to_string();
        std::thread::spawn(move || {
            if let Err(err) = state.run_startup_recipe_steps(&session_id, generation, &steps) {
                log::warn!("startup recipe failed for {session_id}: {err}");
            }
        });
        Ok(())
    }

    fn prepare_startup_recipe_run(
        &self,
        session_id: &str,
    ) -> Result<(u64, Vec<StartupRecipeStep>), String> {
        let (generation, raw_recipe) = {
            let inner = self
                .inner
                .lock()
                .map_err(|_| "state lock poisoned".to_string())?;
            let Some(entry) = inner.sessions.get(session_id) else {
                return Err("session not found".to_string());
            };
            if entry.runtime.is_none() {
                return Err("session is not running".to_string());
            }
            let raw_recipe = entry
                .session
                .startup_command
                .clone()
                .ok_or_else(|| "session has no startup command".to_string())?;
            (entry.generation, raw_recipe)
        };

        let steps = parse_startup_recipe(&raw_recipe)?;
        Ok((generation, steps))
    }

    fn run_startup_recipe_steps(
        &self,
        session_id: &str,
        generation: u64,
        steps: &[StartupRecipeStep],
    ) -> Result<(), String> {
        let mut wait_baseline = self
            .session_recent_output_tail(session_id, generation)
            .unwrap_or_default();

        for step in steps {
            if !self.session_generation_is_running(session_id, generation) {
                return Ok(());
            }

            match step {
                StartupRecipeStep::RunLine(command) => {
                    wait_baseline = self
                        .session_recent_output_tail(session_id, generation)
                        .unwrap_or_default();
                    self.session_input_write(session_id, &format!("{command}\n"))?;
                }
                StartupRecipeStep::TypeText(text) => {
                    wait_baseline = self
                        .session_recent_output_tail(session_id, generation)
                        .unwrap_or_default();
                    self.session_input_write(session_id, text)?;
                }
                StartupRecipeStep::Enter => {
                    wait_baseline = self
                        .session_recent_output_tail(session_id, generation)
                        .unwrap_or_default();
                    self.session_input_write(session_id, "\n")?;
                }
                StartupRecipeStep::SleepMs(ms) => {
                    if !self.wait_startup_recipe_delay(session_id, generation, *ms) {
                        return Ok(());
                    }
                }
                StartupRecipeStep::WaitFor { needle, timeout_ms } => {
                    let Some(current_tail) = self.wait_for_startup_recipe_output(
                        session_id,
                        generation,
                        &wait_baseline,
                        needle,
                        *timeout_ms,
                    )? else {
                        return Ok(());
                    };
                    wait_baseline = current_tail;
                }
            }
        }

        Ok(())
    }

    fn wait_startup_recipe_delay(&self, session_id: &str, generation: u64, ms: u64) -> bool {
        let deadline = Instant::now() + Duration::from_millis(ms);
        loop {
            if !self.session_generation_is_running(session_id, generation) {
                return false;
            }

            let now = Instant::now();
            if now >= deadline {
                return true;
            }

            let remaining = deadline.saturating_duration_since(now);
            std::thread::sleep(remaining.min(Duration::from_millis(
                STARTUP_RECIPE_WAIT_POLL_MS,
            )));
        }
    }

    fn wait_for_startup_recipe_output(
        &self,
        session_id: &str,
        generation: u64,
        baseline: &str,
        needle: &str,
        timeout_ms: u64,
    ) -> Result<Option<String>, String> {
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);

        loop {
            if !self.session_generation_is_running(session_id, generation) {
                return Ok(None);
            }

            let Some(current_tail) = self.session_recent_output_tail(session_id, generation) else {
                return Ok(None);
            };
            if recent_output_delta_contains(baseline, &current_tail, needle) {
                return Ok(Some(current_tail));
            }

            if Instant::now() >= deadline {
                return Err(format!(
                    "startup recipe wait-for timed out after {timeout_ms}ms: {needle}"
                ));
            }

            std::thread::sleep(Duration::from_millis(STARTUP_RECIPE_WAIT_POLL_MS));
        }
    }

    fn session_generation_is_running(&self, session_id: &str, generation: u64) -> bool {
        let Ok(inner) = self.inner.lock() else {
            return false;
        };
        if inner.shutdown_requested {
            return false;
        }
        let Some(entry) = inner.sessions.get(session_id) else {
            return false;
        };
        entry.generation == generation && entry.runtime.is_some()
    }

    fn session_recent_output_tail(&self, session_id: &str, generation: u64) -> Option<String> {
        let inner = self.inner.lock().ok()?;
        let entry = inner.sessions.get(session_id)?;
        if entry.generation != generation || entry.runtime.is_none() {
            return None;
        }
        Some(entry.recent_output_tail.clone())
    }
}

pub(super) fn append_recent_output_tail(buffer: &mut String, chunk: &str) {
    if chunk.is_empty() {
        return;
    }
    buffer.push_str(chunk);
    trim_live_output(buffer, RECENT_OUTPUT_TAIL_MAX_BYTES);
}

fn recent_output_delta_contains(baseline: &str, current: &str, needle: &str) -> bool {
    if let Some(delta) = current.strip_prefix(baseline) {
        return delta.contains(needle);
    }
    current != baseline && current.contains(needle)
}

#[cfg(test)]
mod tests {
    use super::{StartupRecipeStep, parse_startup_recipe};

    #[test]
    fn parse_plain_command_as_single_run_line() {
        let steps = parse_startup_recipe("claude").expect("parse recipe");
        assert_eq!(steps, vec![StartupRecipeStep::RunLine("claude".to_string())]);
    }

    #[test]
    fn parse_multi_step_recipe_with_wait_for() {
        let steps = parse_startup_recipe("run claude; wait 250; wait-for Password: timeout=3000")
            .expect("parse multi-step recipe");
        assert_eq!(
            steps,
            vec![
                StartupRecipeStep::RunLine("claude".to_string()),
                StartupRecipeStep::SleepMs(250),
                StartupRecipeStep::WaitFor {
                    needle: "Password:".to_string(),
                    timeout_ms: 3000,
                },
            ]
        );
    }
}
