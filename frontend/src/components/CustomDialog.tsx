import React from "react";
import { Settings } from "lucide-react";
import { Dialog, DialogContent, DialogTitle, DialogTrigger, DialogFooter } from "./ui/dialog";
import { VisuallyHidden } from "./ui/visually-hidden";
import { t } from "@/i18n/shell";

interface DialogProps {
    triggerComponent: React.ReactElement;
    dialogContent: React.ReactNode;
    dialogTitle?: string;
}

export function CustomDialog({ triggerComponent, dialogContent, dialogTitle = t("dialog") }: DialogProps) {
    return (
        <Dialog>
            <DialogTrigger asChild>
                {triggerComponent}
            </DialogTrigger>
            <DialogContent aria-describedby={undefined}>
                <VisuallyHidden>
                    <DialogTitle>{dialogTitle}</DialogTitle>
                </VisuallyHidden>
                {dialogContent}                  
                <DialogFooter>
                    
                </DialogFooter>
            </DialogContent>
        </Dialog>
    )
}