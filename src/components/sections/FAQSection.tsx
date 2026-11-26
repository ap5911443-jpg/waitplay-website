import React, { useState } from "react";
import { SectionContainer } from "../layout/SectionContainer";
import { Eyebrow } from "../ui/Eyebrow";
import { EditorialHeading } from "../ui/EditorialHeading";
import { AccordionItem } from "../ui/AccordionItem";
import { FAQS } from "../../data/faqs";

export const FAQSection: React.FC = () => {
  const [openId, setOpenId] = useState<string | null>("how-it-works");

  const handleToggle = (id: string) => {
    setOpenId((prev) => (prev === id ? null : id));
  };

  return (
    <SectionContainer id="faq" bg="canvas" className="overflow-hidden">
      <div className="max-w-[880px] mx-auto">
        <div className="text-center mb-12 sm:mb-16">
          <Eyebrow variant="lavender" dot className="mb-4">
            FREQUENTLY ASKED QUESTIONS
          </Eyebrow>

          <EditorialHeading level="section" className="mb-4">
            Good <em className="italic font-normal">questions.</em>
          </EditorialHeading>

          <p className="font-sans text-[17px] sm:text-[19px] text-[#666666] leading-[1.6]">
            Everything you need to know about Vibe Play's local bridge, agent integration hooks, and privacy model.
          </p>
        </div>

        {/* Accordion List */}
        <div className="border-t border-[rgba(26,26,26,0.12)]">
          {FAQS.map((faq) => (
            <AccordionItem
              key={faq.id}
              id={faq.id}
              question={faq.question}
              answer={faq.answer}
              isOpen={openId === faq.id}
              onToggle={() => handleToggle(faq.id)}
            />
          ))}
        </div>
      </div>
    </SectionContainer>
  );
};
