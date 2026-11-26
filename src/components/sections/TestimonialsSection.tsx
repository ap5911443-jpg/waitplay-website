import React from "react";
import { SectionContainer } from "../layout/SectionContainer";
import { Eyebrow } from "../ui/Eyebrow";
import { EditorialHeading } from "../ui/EditorialHeading";
import { TESTIMONIALS } from "../../data/testimonials";

export const TestimonialsSection: React.FC = () => {
  return (
    <SectionContainer id="reviews" bg="canvas" className="overflow-hidden">
      <div className="text-center max-w-[800px] mx-auto mb-14 sm:mb-20">
        <Eyebrow variant="lavender" dot className="mb-4">
          DEVELOPER EXPERIENCES
        </Eyebrow>

        <EditorialHeading level="section" className="mb-5">
          From developers <em className="italic font-normal">who ship with agents.</em>
        </EditorialHeading>

        <p className="font-sans text-[17px] sm:text-[19px] text-[#666666] leading-[1.6]">
          Engineers building with Claude Code, Cursor, and OMP share how Vibe Play eliminated prompt-downtime context fractures.
        </p>
      </div>

      {/* Testimonials Grid */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-6 sm:gap-8 items-stretch">
        {TESTIMONIALS.map((t) => (
          <div
            key={t.id}
            className="p-7 sm:p-8 rounded-[24px] bg-[#FFFFFF] border border-[rgba(26,26,26,0.12)] shadow-card flex flex-col justify-between hover:-translate-y-1 transition-transform duration-200"
          >
            <div>
              {/* Stat callout if available */}
              {t.stats && (
                <div className="mb-5 pb-4 border-b border-[rgba(26,26,26,0.08)] flex items-baseline gap-2">
                  <span className="font-serif text-[32px] font-bold text-[#034F46] leading-none">
                    {t.stats.value}
                  </span>
                  <span className="font-sans text-[12px] uppercase tracking-wider text-[#888888] font-semibold">
                    {t.stats.label}
                  </span>
                </div>
              )}

              {/* Quote text in Garamond serif */}
              <p className="font-serif text-[18px] sm:text-[20px] text-[#1A1A1A] leading-[1.4] italic mb-6">
                "{t.quote}"
              </p>
            </div>

            {/* Author info in Figtree sans */}
            <div className="pt-4 border-t border-[rgba(26,26,26,0.08)]">
              <div className="font-sans font-bold text-[15px] text-[#1A1A1A]">
                {t.author}
              </div>
              <div className="font-sans text-[13px] text-[#666666]">
                {t.role}, <span className="text-[#1A1A1A]">{t.company}</span>
              </div>
            </div>
          </div>
        ))}
      </div>
    </SectionContainer>
  );
};
