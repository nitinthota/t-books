import { cva, type VariantProps } from "class-variance-authority";
import { type ButtonHTMLAttributes, forwardRef } from "react";
import { cn } from "@/lib/utils";

const buttonVariants = cva(
  "pressable inline-flex items-center justify-center gap-2 rounded-md font-medium transition-[background-color,color,box-shadow,opacity] duration-160 ease-out disabled:cursor-not-allowed disabled:opacity-50",
  {
    variants: {
      variant: {
        primary: "bg-navy text-paper-raised hover:bg-navy-hover",
        secondary:
          "bg-paper-raised text-ink shadow-[0_0_0_1px_var(--color-line)] hover:bg-paper-sunken",
        ghost: "bg-transparent text-ink-muted hover:bg-paper-sunken hover:text-ink",
        danger: "bg-danger text-paper-raised hover:opacity-90",
      },
      size: {
        md: "h-11 px-4 text-sm",
        sm: "h-9 px-3 text-sm",
        lg: "h-12 px-5 text-base",
      },
    },
    defaultVariants: {
      variant: "primary",
      size: "md",
    },
  },
);

export type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> &
  VariantProps<typeof buttonVariants>;

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button(
  { className, variant, size, type = "button", ...props },
  ref,
) {
  return (
    <button
      ref={ref}
      type={type}
      className={cn(buttonVariants({ variant, size }), className)}
      {...props}
    />
  );
});
