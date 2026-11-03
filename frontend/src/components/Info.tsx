import React from "react";
import { Info as InfoIcon } from "lucide-react";
import { Dialog, DialogContent, DialogTitle, DialogTrigger } from "./ui/dialog";
import { Tooltip, TooltipContent, TooltipTrigger } from "./ui/tooltip";
import { About } from "./About";
import { t } from "@/i18n/shell";

interface InfoProps {
    isCollapsed: boolean;
}

// Same look as the other sidebar controls.
const railFocus = "transition-colors duration-150 hover:bg-rail-active focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-rail-accent";

const Info = React.forwardRef<HTMLButtonElement, InfoProps>(({ isCollapsed }, ref) => {
  const trigger = (
    <DialogTrigger asChild>
      <button
        ref={ref}
        type="button"
        aria-label={isCollapsed ? t("info") : undefined}
        className={
          isCollapsed
            ? `flex h-8 w-8 items-center justify-center rounded-sm text-rail-muted hover:text-rail-foreground ${railFocus}`
            : `flex h-8 w-full items-center gap-2 rounded-sm px-2 text-sm text-rail-foreground ${railFocus}`
        }
      >
        <InfoIcon className="h-4 w-4 shrink-0 text-rail-muted" />
        {!isCollapsed && t("info")}
      </button>
    </DialogTrigger>
  );

  return (
    <Dialog>
      {isCollapsed ? (
        <Tooltip>
          <TooltipTrigger asChild>{trigger}</TooltipTrigger>
          <TooltipContent side="right">{t("info")}</TooltipContent>
        </Tooltip>
      ) : (
        trigger
      )}
      <DialogContent className="sm:max-w-sm" aria-describedby={undefined}>
        <DialogTitle className="sr-only">{t("aboutTitle")}</DialogTitle>
        <About />
      </DialogContent>
    </Dialog>
  );
});

Info.displayName = "Info";

export default Info;
