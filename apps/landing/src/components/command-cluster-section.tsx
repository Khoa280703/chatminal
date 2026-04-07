import { commandSteps } from "@/lib/landing-data";

export function CommandClusterSection() {
  return (
    <section className="mx-auto mt-32 max-w-7xl px-6">
      <div className="flex flex-col items-center gap-16 md:flex-row">
        <div className="w-full md:w-1/2">
          <h2 className="mb-8 font-headline text-4xl font-bold leading-tight tracking-tight text-white md:text-6xl">
            COMMAND YOUR
            <br />
            AI CLUSTER.
          </h2>
          <p className="mb-8 font-mono leading-relaxed text-[#a0a0a0]">
            Vibe coding is not just chatting. It is orchestration. Use CLI
            commands to distribute tasks across your agent tree.
          </p>
          <ul className="space-y-4 font-mono text-sm">
            {commandSteps.map((command, index) => (
              <li key={command} className="flex items-start gap-3">
                <span className="font-bold text-white">{String(index + 1).padStart(2, "0")}</span>
                <span className="text-terminal-text">{command}</span>
              </li>
            ))}
          </ul>
        </div>

        <div className="w-full md:w-1/2">
          <div className="relative overflow-hidden border border-white/10 bg-white/5 p-8 font-mono text-sm">
            <div className="absolute right-0 top-0 p-4 opacity-5">
              <span className="font-headline text-[100px] text-white">&lt;/&gt;</span>
            </div>
            <div className="mb-4 border-b border-white/10 pb-2 text-white">
              {"// Vibe Sync Protocol"}
            </div>
            <div className="space-y-1 leading-7">
              <div className="text-sky-300">
                import {"{"} AgentTree {"}"} from {'@chatminal/sdk'};
              </div>
              <br />
              <div className="text-terminal-text">
                const cluster = new AgentTree({'vibe_v1'});
              </div>
              <div className="text-[#a0a0a0]">cluster.assign({"{"})</div>
              <div className="pl-4 text-[#a0a0a0]">role: {'Architect'},</div>
              <div className="pl-4 text-[#a0a0a0]">behavior: {'Aggressive_Refactor'},</div>
              <div className="pl-4 text-[#a0a0a0]">context: {'./src/lib'}</div>
              <div className="text-[#a0a0a0]">{"});"}</div>
              <br />
              <div className="text-green-300">await cluster.vibeCheck();</div>
              <div className="text-terminal-text">cluster.execute();</div>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
