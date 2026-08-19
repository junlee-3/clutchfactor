import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  buildCorpus,
  corpusStatus,
  getGrid,
  getHabits,
  getMatchDetail,
  getMatchReport,
  getRoundTicks,
  importCorpusDemo,
  importDemo,
  listMatches,
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
