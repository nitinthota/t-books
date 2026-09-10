import { memo } from "react";

export const ScreenSkeleton = memo(function ScreenSkeleton() {
  return (
    <div className="enter space-y-4" aria-busy="true" aria-live="polite">
      <div className="skeleton h-4 w-16" />
      <div className="skeleton h-9 w-40" />
      <div className="skeleton h-4 w-72 max-w-full" />
      <div className="mt-6 space-y-2">
        <div className="skeleton h-11 w-full" />
        <div className="skeleton h-11 w-full" />
        <div className="skeleton h-11 w-full" />
        <div className="skeleton h-11 w-5/6" />
      </div>
    </div>
  );
});
