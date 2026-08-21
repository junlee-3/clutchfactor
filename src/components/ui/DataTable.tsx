import type { ReactNode } from "react";

interface DataTableProps {
  head: string[];
  rows: ReactNode[][];
  rowKey: (rowIndex: number) => string;
  onRowClick?: (rowIndex: number) => void;
}

// The one table style (design-system.md §6): sans header eyebrows, mono
// tabular data cells, hairline rows. When onRowClick is given, the row's
// first cell renders as a <button> (not the whole <tr>, which isn't a valid
// interactive element) so the row stays a single, real, keyboard-focusable
// control per a11y.
export function DataTable({ head, rows, rowKey, onRowClick }: DataTableProps) {
  return (
    <table className="ui-table">
      <thead>
        <tr>
          {head.map((label, i) => (
            <th key={i} className="type-micro">
              {label}
            </th>
          ))}
        </tr>
      </thead>
      <tbody>
        {rows.map((cells, rowIndex) => (
          <tr key={rowKey(rowIndex)}>
            {cells.map((cell, cellIndex) =>
              onRowClick && cellIndex === 0 ? (
                <td key={cellIndex}>
                  <button
                    type="button"
                    className="ui-table-row-btn"
                    onClick={() => onRowClick(rowIndex)}
                  >
                    {cell}
                  </button>
                </td>
              ) : (
                <td key={cellIndex}>{cell}</td>
              ),
            )}
          </tr>
        ))}
      </tbody>
    </table>
  );
}
