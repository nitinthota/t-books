import { type InputHTMLAttributes, forwardRef } from "react";
import { cn } from "@/lib/utils";

export const Input = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement>>(
  function Input({ className, ...props }, ref) {
    return (
      <input
        ref={ref}
        className={cn(
          "h-11 w-full rounded-md bg-paper-raised px-3 text-base text-ink ring-1 ring-line",
          "placeholder:text-ink-subtle",
          "transition-[box-shadow] duration-160 ease-out",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-navy",
          "disabled:opacity-60",
          className,
        )}
        {...props}
      />
    );
  },
);
