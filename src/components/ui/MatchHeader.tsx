import type { ReactNode } from "react";
import { Link } from "react-router-dom";
import { mapName } from "../../lib/mapName";

export type MatchResult = "win" | "loss" | "tie";

export interface MatchHeaderLink {
  to: string;
  label: string;
}

export interface MatchHeaderProps {
  map: string;
  score: { a: number; b: number };
  result?: MatchResult | null;
  /** Match/import date, pre-formatted. Omit when the caller's loaded data
   *  doesn't carry one (no MatchHeader field is required — §8 degrades). */
  date?: string | null;
  /** V1.4: the StatsStrip. Absent only until a caller's stats resolve —
   *  the slot still renders (as a placeholder) so the header never shifts. */
  strip?: ReactNode;
  back: MatchHeaderLink;
  crossLink?: MatchHeaderLink;
}

const RESULT_LABEL: Record<MatchResult, string> = {
  win: "WON",
  loss: "LOST",
  tie: "TIE",
};

// Reused by Report and Replay (design-system.md §8, charter-mandated):
// map (display-sans title) · score · result (game hues) · date · the stats
// strip, plus back-navigation and the Report<->Replay cross-link. Every
// field but map/score/back is optional — a screen whose loaded data lacks a
// stat still renders that slot (as a placeholder, see below) rather than
// fetching more to fill the mock.
export function MatchHeader({ map, score, result, date, strip, back, crossLink }: MatchHeaderProps) {
  return (
    <header className="match-header">
      <Link to={back.to} className="match-header-back">
        {back.label}
      </Link>
      <h1 className="match-header-map type-title">{mapName(map)}</h1>
      <span className="match-header-score type-data">
        {score.a} : {score.b}
      </span>
      {/* result/date/stats are optional (the Report/Replay join they come
          from can resolve after first paint on a cold deep link) but always
          render in their fixed-width slot so late-arriving data doesn't
          shift the header layout (design-system.md §10: "no layout shift on
          data arrival"). */}
      <span
        className={`match-header-result${
          result ? ` match-header-result-${result}` : " match-header-pending"
        }`}
      >
        {result ? RESULT_LABEL[result] : "—"}
      </span>
      <span className={`match-header-date type-data${date ? "" : " match-header-pending"}`}>
        {date ?? "—"}
      </span>
      {strip ?? (
        <span className="match-header-stats type-data match-header-pending">—</span>
      )}
      {crossLink && (
        <Link to={crossLink.to} className="match-header-cross">
          {crossLink.label}
        </Link>
      )}
    </header>
  );
}
