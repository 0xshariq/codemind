"use client";

import { Dialog } from "@base-ui/react/dialog";
import type { ComponentProps } from "react";
import { X } from "lucide-react";
import { cn } from "@/lib/utils";

export function Modal(props: Dialog.Root.Props) { return <Dialog.Root {...props} />; }
export function ModalTrigger(props: Dialog.Trigger.Props) { return <Dialog.Trigger {...props} />; }
export function ModalClose(props: Dialog.Close.Props) { return <Dialog.Close {...props} />; }

export function ModalContent({ className, children, ...props }: Dialog.Popup.Props) {
  return <Dialog.Portal>
    <Dialog.Backdrop className="fixed inset-0 z-50 bg-black/60 backdrop-blur-sm data-ending-style:opacity-0 data-starting-style:opacity-0" />
    <Dialog.Popup className={cn("fixed left-1/2 top-1/2 z-50 grid max-h-[min(85svh,48rem)] w-[min(calc(100vw-2rem),40rem)] -translate-x-1/2 -translate-y-1/2 gap-4 overflow-y-auto rounded-xl border border-border bg-popover p-5 text-popover-foreground shadow-2xl outline-none data-ending-style:scale-95 data-starting-style:scale-95", className)} {...props}>
      {children}
      <Dialog.Close aria-label="Close dialog" className="absolute right-3 top-3 inline-grid size-9 place-items-center rounded-md text-muted-foreground hover:bg-surface-alt hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"><X aria-hidden="true" className="size-4" /></Dialog.Close>
    </Dialog.Popup>
  </Dialog.Portal>;
}

export function ModalHeader({ className, ...props }: ComponentProps<"div">) {
  return <div className={cn("grid gap-1.5 pr-9", className)} {...props} />;
}
export function ModalTitle({ className, ...props }: Dialog.Title.Props) {
  return <Dialog.Title className={cn("text-lg font-semibold tracking-tight", className)} {...props} />;
}
export function ModalDescription({ className, ...props }: Dialog.Description.Props) {
  return <Dialog.Description className={cn("text-sm leading-6 text-muted-foreground", className)} {...props} />;
}
