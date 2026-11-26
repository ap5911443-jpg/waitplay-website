import React from "react";

export const TestimonialsSection: React.FC = () => {
  const testimonials = [
    {
      quote:
        "“Waiting 45 seconds for Claude Code to refactor a file used to destroy my momentum. I’d open Twitter and lose 20 minutes. Vibe Play gives me an instant brain break and drops me right back into terminal flow.”",
      author: "Alex Rivers",
      role: "Staff Infrastructure Engineer",
      team: "Multi-Agent Systems",
    },
    {
      quote:
        "“The Wispr Flow desktop widget is pure genius. When I'm running multi-terminal agents in Windows Terminal, I can mute the interface in one click so it never interrupts my second agent.”",
      author: "Elena Rostova",
      role: "Lead Backend Developer",
      team: "Autonomous Services",
    },
  ];

  return (
    <section className="w-full py-20 px-4 sm:px-8 bg-canvas border-t border-ink/10">
      <div className="max-w-5xl mx-auto space-y-12">
        <div className="text-center space-y-2">
          <span className="text-xs font-sans font-semibold tracking-wider uppercase text-forest">
            Developer Perspective
          </span>
          <h2 className="font-serif font-normal text-4xl sm:text-5xl text-ink">
            Designed for engineers who build with agents all day.
          </h2>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-8 text-left">
          {testimonials.map((item, i) => (
            <div
              key={i}
              className="bg-white rounded-3xl p-8 border border-ink/15 shadow-subtle flex flex-col justify-between space-y-6"
            >
              <p className="font-serif text-xl sm:text-2xl text-ink font-normal leading-relaxed">
                {item.quote}
              </p>
              <div className="border-t border-ink/10 pt-4">
                <div className="font-sans font-semibold text-sm text-ink">{item.author}</div>
                <div className="text-xs font-sans text-ink-muted">
                  {item.role} • {item.team}
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
};
