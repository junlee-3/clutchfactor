import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  buildCorpus,
  coachStatus,
  corpusStatus,
  deleteMatch,
  getAppSettings,
  getCoachRounds,
  getCoachSynthesis,
  getDetectorCatalog,
  getGrid,
  getHabits,
  getMapCallouts,
  getMatchDetail,
  getMatchReport,
  getMatchStats,
  getRoundReview,
  getRoundScoreboard,
  getRoundTicks,
  getTrends,
  importCorpusDemo,
  importDemo,
  listMatches,
  reAnalyzeMatch,
  regenerateCoachRound,
  regenerateCoachSynthesis,
  setCoachEnabled,
  setCoachModels,
  setGeminiKey,
  setTrackedOverride,
  testGeminiKey,
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
    onSuccess: (result) => {
      void client.invalidateQueries({ queryKey: ["matches"] });
      void client.invalidateQueries({ queryKey: ["tracked_player"] });
      void client.invalidateQueries({
        queryKey: ["match_stats", result.match_id],
      });
      void client.invalidateQueries({
        queryKey: ["scoreboard", result.match_id],
      });
      void client.invalidateQueries({ queryKey: ["trends"] });
      void client.invalidateQueries({ queryKey: ["map_callouts"] });
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
      void client.invalidateQueries({ queryKey: ["match_stats", matchId] });
      void client.invalidateQueries({ queryKey: ["scoreboard", matchId] });
      void client.invalidateQueries({ queryKey: ["map_callouts"] });
      // A re-parse changes the facts: the coach cache hash handles the
      // regeneration, but the UI must refetch.
      void client.invalidateQueries({ queryKey: ["coach_rounds", matchId] });
      void client.invalidateQueries({ queryKey: ["coach_synthesis", matchId] });
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

// ---- V1.3: the coach ----

export function useCoachStatus() {
  return useQuery({ queryKey: ["coach_status"], queryFn: coachStatus });
}

export function useCoachRounds(matchId: number, enabled: boolean) {
  return useQuery({
    queryKey: ["coach_rounds", matchId],
    queryFn: () => getCoachRounds(matchId),
    enabled,
    staleTime: Infinity,
  });
}

export function useCoachSynthesis(matchId: number, enabled: boolean) {
  return useQuery({
    queryKey: ["coach_synthesis", matchId],
    queryFn: () => getCoachSynthesis(matchId),
    enabled,
    staleTime: Infinity,
  });
}

function useCoachSettingMutation<T>(fn: (v: T) => Promise<void>) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: fn,
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ["coach_status"] });
      void client.invalidateQueries({ queryKey: ["coach_rounds"] });
      void client.invalidateQueries({ queryKey: ["coach_synthesis"] });
    },
  });
}

export function useSetGeminiKey() {
  return useCoachSettingMutation((key: string | null) => setGeminiKey(key));
}

export function useSetCoachModels() {
  return useCoachSettingMutation(
    ({ roundModel, synthesisModel }: { roundModel: string; synthesisModel: string }) =>
      setCoachModels(roundModel, synthesisModel),
  );
}

export function useSetCoachEnabled() {
  return useCoachSettingMutation((enabled: boolean) => setCoachEnabled(enabled));
}

export function useTestGeminiKey() {
  return useMutation({ mutationFn: () => testGeminiKey() });
}

export function useRegenerateCoachRound() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: ({ matchId, round }: { matchId: number; round: number }) =>
      regenerateCoachRound(matchId, round),
    onSuccess: (data, { matchId }) => {
      client.setQueryData(["coach_rounds", matchId], data);
      void client.invalidateQueries({ queryKey: ["coach_synthesis", matchId] });
    },
  });
}

export function useRegenerateCoachSynthesis() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (matchId: number) => regenerateCoachSynthesis(matchId),
    onSuccess: (data, matchId) => {
      client.setQueryData(["coach_synthesis", matchId], data);
    },
  });
}

// ---- V1.4: stats & understanding ----

export function useMatchStats(matchId: number) {
  return useQuery({
    queryKey: ["match_stats", matchId],
    queryFn: () => getMatchStats(matchId),
  });
}

export function useRoundScoreboard(matchId: number, round: number | null) {
  return useQuery({
    queryKey: ["scoreboard", matchId, round],
    queryFn: () => getRoundScoreboard(matchId, round),
  });
}

export function useDetectorCatalog() {
  return useQuery({ queryKey: ["catalog"], queryFn: getDetectorCatalog });
}

export function useMapCallouts(map: string | null) {
  return useQuery({
    queryKey: ["map_callouts", map],
    queryFn: () => getMapCallouts(map as string),
    enabled: map !== null,
  });
}
