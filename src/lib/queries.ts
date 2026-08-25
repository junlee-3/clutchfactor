import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  buildCorpus,
  corpusStatus,
  deleteMatch,
  getAppSettings,
  getGrid,
  getHabits,
  getMatchDetail,
  getMatchReport,
  getRoundReview,
  getRoundTicks,
  getTrends,
  importCorpusDemo,
  importDemo,
  listMatches,
  reAnalyzeMatch,
  setTrackedOverride,
  trackedPlayer,
} from "./ipc";
import type { ProgressEvent } from "./ipc";

export function useMatchReport(matchId: number) {
  return useQuery({
    queryKey: ["report", matchId],
    queryFn: () => getMatchReport(matchId),
  });
}

export function useHabits() {
  return useQuery({ queryKey: ["habits"], queryFn: getHabits });
}

export function useTrends() {
  return useQuery({ queryKey: ["trends"], queryFn: getTrends });
}

export function useMatchDetail(matchId: number) {
  return useQuery({
    queryKey: ["match", matchId],
    queryFn: () => getMatchDetail(matchId),
  });
}

export function useRoundTicks(matchId: number, round: number) {
  return useQuery({
    queryKey: ["ticks", matchId, round],
    queryFn: () => getRoundTicks(matchId, round),
    staleTime: Infinity, // demo data is immutable once imported
  });
}

export function useRoundReview(matchId: number) {
  return useQuery({
    queryKey: ["round_review", matchId],
    queryFn: () => getRoundReview(matchId),
    staleTime: Infinity, // demo data is immutable once imported
  });
}

export function useMatches() {
  return useQuery({ queryKey: ["matches"], queryFn: listMatches });
}

export function useTrackedPlayer() {
  return useQuery({ queryKey: ["tracked_player"], queryFn: trackedPlayer });
}

export function useImportDemo(onProgress: (e: ProgressEvent) => void) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (path: string) => importDemo(path, onProgress),
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ["matches"] });
      void client.invalidateQueries({ queryKey: ["tracked_player"] });
    },
  });
}

// ---- M5: reference corpus ----

export function useCorpusStatus() {
  return useQuery({ queryKey: ["corpus_status"], queryFn: corpusStatus });
}

export function useGrid(
  map: string | null,
  side: "CT" | "T",
  phase: string,
) {
  return useQuery({
    queryKey: ["grid", map, side, phase],
    queryFn: () => getGrid(map as string, side, phase),
    enabled: map !== null,
  });
}

export function useImportCorpusDemo(onProgress: (e: ProgressEvent) => void) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (path: string) => importCorpusDemo(path, onProgress),
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ["corpus_status"] });
    },
  });
}

export function useBuildCorpus(onProgress: (e: ProgressEvent) => void) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (map: string | null) => buildCorpus(map, onProgress),
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ["corpus_status"] });
      void client.invalidateQueries({ queryKey: ["grid"] });
      // D6 insights may change after a rebuild.
      void client.invalidateQueries({ queryKey: ["report"] });
    },
  });
}

// ---- M6: settings + housekeeping ----

export function useAppSettings() {
  return useQuery({ queryKey: ["app_settings"], queryFn: getAppSettings });
}

export function useSetTrackedOverride() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (steamid: string | null) => setTrackedOverride(steamid),
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ["app_settings"] });
      void client.invalidateQueries({ queryKey: ["tracked_player"] });
    },
  });
}

export function useReAnalyzeMatch(onProgress: (e: ProgressEvent) => void) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: ({ matchId, path }: { matchId: number; path: string | null }) =>
      reAnalyzeMatch(matchId, path, onProgress),
    onSuccess: (result, { matchId }) => {
      if (result.needs_file) return; // nothing changed yet
      void client.invalidateQueries({ queryKey: ["matches"] });
      void client.invalidateQueries({ queryKey: ["match", matchId] });
      void client.invalidateQueries({ queryKey: ["report", matchId] });
      void client.invalidateQueries({ queryKey: ["round_review", matchId] });
      void client.invalidateQueries({ queryKey: ["ticks", matchId] });
      void client.invalidateQueries({ queryKey: ["habits"] });
      void client.invalidateQueries({ queryKey: ["trends"] });
    },
  });
}

export function useDeleteMatch() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (matchId: number) => deleteMatch(matchId),
    onSuccess: () => {
      // Every cross-match surface can change when a match disappears.
      void client.invalidateQueries({ queryKey: ["matches"] });
      void client.invalidateQueries({ queryKey: ["habits"] });
      void client.invalidateQueries({ queryKey: ["trends"] });
      void client.invalidateQueries({ queryKey: ["app_settings"] });
      void client.invalidateQueries({ queryKey: ["tracked_player"] });
    },
  });
}
