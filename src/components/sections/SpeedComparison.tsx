import React from "react";
import { SectionContainer } from "../layout/SectionContainer";
import { Eyebrow } from "../ui/Eyebrow";
import { EditorialHeading } from "../ui/EditorialHeading";
import { InteractiveSpeedSlider } from "../interactive/InteractiveSpeedSlider";

export const SpeedComparison: React.FC = () => {
  return (
    <SectionContainer id="speed" bg="forest" className="relative overflow-hidden">
      <div className="text-center max-w-[840px] mx-auto mb-12 sm:mb-16">
        <Eyebrow variant="forest" dot className="mb-4">
          FOCUS RECOVERY METRICS
        </Eyebrow>

        <EditorialHeading level="section" theme="forest" className="mb-5">
          45 seconds of dead air, <em className="italic font-normal text-[#F0D7FF]">turned into pure flow.</em>
        </EditorialHeading>

        <p className="font-sans text-[17px] sm:text-[19px] text-[#FFFFEB]/80 leading-[1.6] font-normal">
          Waiting for an AI agent to inspect files, execute bash tools, and run test suites usually triggers subconscious tab-switching. You lose 15 minutes of productive momentum. Vibe Play keeps your brain in the zone.
        </p>
      </div>

      {/* Interactive Comparison Component */}
      <InteractiveSpeedSlider />
    </SectionContainer>
  );
};
