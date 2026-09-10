import { type ReactNode, useState } from "react";
import { cn } from "@/lib/utils";

export function VirtualTable<T>({
  rows,
  rowKey,
  header,
  renderRow,
  empty,
  rowHeight = 44,
}: {
  rows: T[];
  rowKey: (row: T, index: number) => string | number;
  header: ReactNode;
  renderRow: (row: T, index: number) => ReactNode;
  empty: ReactNode;
  rowHeight?: number;
}) {
  const [scrollTop, setScrollTop] = useState(0);
  if (rows.length === 0) {
    return (
      <div className="overflow-x-auto">
        <table className="w-full min-w-[36rem] text-left text-sm">
          <thead className="border-b border-line text-xs font-medium uppercase tracking-wide text-ink-subtle">
            {header}
          </thead>
        </table>
        <div className="px-5 py-8">{empty}</div>
      </div>
    );
  }
  const virtualize = rows.length > 40;
  if (!virtualize) {
    return (
      <div className="overflow-x-auto">
        <table className="w-full min-w-[36rem] text-left text-sm">
          <thead className="border-b border-line text-xs font-medium uppercase tracking-wide text-ink-subtle">
            {header}
          </thead>
          <tbody>
            {rows.map((row, index) => (
              <RowWrap key={rowKey(row, index)}>{renderRow(row, index)}</RowWrap>
            ))}
          </tbody>
        </table>
      </div>
    );
  }
  const viewport = 384;
  const start = Math.max(0, Math.floor(scrollTop / rowHeight) - 8);
  const visible = Math.ceil(viewport / rowHeight) + 16;
  const slice = rows.slice(start, start + visible);
  const padTop = start * rowHeight;
  const padBottom = Math.max(0, (rows.length - start - slice.length) * rowHeight);
  return (
    <div
      className="max-h-96 overflow-auto"
      onScroll={(event) => setScrollTop(event.currentTarget.scrollTop)}
    >
      <table className="w-full min-w-[36rem] text-left text-sm">
        <thead className="sticky top-0 z-10 border-b border-line bg-paper-raised text-xs font-medium uppercase tracking-wide text-ink-subtle">
          {header}
        </thead>
        <tbody>
          {padTop > 0 ? (
            <tr aria-hidden="true">
              <td className="p-0" colSpan={12} style={{ height: padTop }} />
            </tr>
          ) : null}
          {slice.map((row, i) => (
            <RowWrap key={rowKey(row, start + i)}>{renderRow(row, start + i)}</RowWrap>
          ))}
          {padBottom > 0 ? (
            <tr aria-hidden="true">
              <td className="p-0" colSpan={12} style={{ height: padBottom }} />
            </tr>
          ) : null}
        </tbody>
      </table>
    </div>
  );
}

function RowWrap({ children }: { children: ReactNode }) {
  return <>{children}</>;
}

export function TableHeadCell({
  children,
  className,
}: {
  children?: ReactNode;
  className?: string;
}) {
  return <th className={cn("px-4 py-3 font-medium", className)}>{children}</th>;
}

export function TableCell({
  children,
  className,
  title,
}: {
  children?: ReactNode;
  className?: string;
  title?: string;
}) {
  return (
    <td className={cn("px-4 py-3 text-ink", className)} title={title}>
      {children}
    </td>
  );
}
