import { TerminalWindowPreview } from "@/components/terminal-window-preview";

export function HeroSection() {
  return (
    <section className="mx-auto mt-12 flex max-w-7xl flex-col items-center px-6 text-center">
      <h1 className="text-balance mb-8 max-w-4xl font-headline text-5xl font-bold leading-none tracking-tight text-white md:text-8xl">
        A Tree-based Terminal for Vibe Coding.
      </h1>
      <p className="mb-10 max-w-2xl font-mono text-lg text-[#a0a0a0]">
        Orchestrate multiple AI agents through a structured tree hierarchy.
        Fluid, focused, and engineered for the speed of thought.
      </p>
      <TerminalWindowPreview />
    </section>
  );
}
