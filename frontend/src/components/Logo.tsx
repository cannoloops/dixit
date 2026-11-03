import React from "react";
import { useRouter } from "next/navigation";
import { DixitMark } from "@/components/dixit/DixitMark";
import { Tooltip, TooltipContent, TooltipTrigger } from "./ui/tooltip";
import { t } from "@/i18n/shell";

interface LogoProps {
  isCollapsed: boolean;
}

/** Dixit mark (and name when the rail is expanded): goes to the home page. */
const Logo = React.forwardRef<HTMLButtonElement, LogoProps>(
  ({ isCollapsed }, ref) => {
    const router = useRouter();
    const button = (
      <button
        ref={ref}
        type="button"
        onClick={() => router.push("/")}
        aria-label={t("home")}
        className="flex h-8 items-center gap-2 rounded-sm px-1 transition-opacity duration-150 hover:opacity-80 focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-rail-accent"
      >
        <DixitMark className="h-6 w-6 text-base text-rail-accent" />
        {!isCollapsed && (
          <span className="text-sm font-semibold text-rail-foreground">Dixit</span>
        )}
      </button>
    );

    if (!isCollapsed) return button;
    return (
      <Tooltip>
        <TooltipTrigger asChild>{button}</TooltipTrigger>
        <TooltipContent side="right">{t("home")}</TooltipContent>
      </Tooltip>
    );
  },
);

Logo.displayName = "Logo";

export default Logo;
