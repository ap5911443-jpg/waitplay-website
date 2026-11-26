import { TestimonialItem } from "../types";

export const TESTIMONIALS: TestimonialItem[] = [
  {
    id: "steven-b",
    quote:
      "I used to unconsciously open Twitter or Slack every single time Claude Code ran bash commands or tool chains. Vibe Play keeps my brain primed in a calm micro-flow while the agent writes tests. It's completely eliminated my mid-prompt context loss.",
    author: "Steven Bartlett",
    role: "Lead Systems Architect",
    company: "Autonomous Engineering Lab",
    stats: {
      value: "42 min",
      label: "recovered focus per day",
    },
  },
  {
    id: "elena-v",
    quote:
      "The Wispr Flow style floating pill widget docked to the edge of my IDE is sheer genius. It gives me a tranquil pulse when my agent is reasoning, and never steals focus abruptly when it's done.",
    author: "Elena Rostova",
    role: "Senior Staff Engineer",
    company: "CloudScale Systems",
    stats: {
      value: "100%",
      label: "local & loopback secured",
    },
  },
  {
    id: "marcus-k",
    quote:
      "Vibe Play's zero-interruption return is what separates it from gimmicks. When an agent run finishes, the completion card quietly informs you without hijacking your keystrokes. It's the highest quality companion tool for agentic developers.",
    author: "Marcus Chen",
    role: "Core Contributor",
    company: "OpenAgent Protocol",
    stats: {
      value: "< 1s",
      label: "instant transition back to code",
    },
  },
];
