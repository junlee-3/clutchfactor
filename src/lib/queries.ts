import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { importDemo, listMatches, trackedPlayer } from "./ipc";
import type { ProgressEvent } from "./ipc";

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
