import { ChevronLeft } from "lucide-react";
import { memo, type ReactNode } from "react";
import { ErrorBanner } from "./error-banner";

export const ModuleFrame = memo(function ModuleFrame({
  title,
  hint,
  onBack,
  actions,
  error,
  children,
}: {
  title: string;
  hint?: string;
  onBack: () => void;
  actions?: ReactNode;
  error?: string | null;
  children: ReactNode;
}) {
  return (
    <section className="enter">
      <button
        type="button"
        className="pressable mb-5 inline-flex h-11 items-center gap-1 text-sm text-ink-muted hover:text-ink"
        onClick={onBack}
      >
        <ChevronLeft className="size-4" strokeWidth={1.75} aria-hidden="true" />
        Back
      </button>
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <h1 className="text-3xl font-medium tracking-tight">{title}</h1>
          {hint ? <p className="mt-1.5 max-w-xl text-sm text-ink-muted">{hint}</p> : null}
        </div>
        {actions ? <div className="flex flex-wrap gap-2">{actions}</div> : null}
      </div>
      {error ? (
        <div className="mt-4">
          <ErrorBanner message={error} />
        </div>
      ) : null}
      <div className="mt-6">{children}</div>
    </section>
  );
});

export function SuggestField({
  id,
  label,
  value,
  onChange,
  options,
  placeholder,
  required,
}: {
  id: string;
  label: string;
  value: string;
  onChange: (value: string) => void;
  options: string[];
  placeholder?: string;
  required?: boolean;
}) {
  const listId = `${id}-list`;
  return (
    <div>
      <label htmlFor={id} className="block text-sm font-medium text-ink-muted">
        {label}
        {required ? <span className="text-danger"> *</span> : null}
      </label>
      <input
        id={id}
        list={listId}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className="mt-1.5 h-11 w-full rounded-md bg-paper-raised px-3 text-base text-ink ring-1 ring-line placeholder:text-ink-subtle focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-navy"
      />
      <datalist id={listId}>
        {options.map((option) => (
          <option key={option} value={option} />
        ))}
      </datalist>
    </div>
  );
}
