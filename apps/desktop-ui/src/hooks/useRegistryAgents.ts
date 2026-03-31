import { useCallback, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  RegistryAgent,
  paginatedRegistryAgentsSchema,
} from "../types/agent-registry";

interface UseRegistryAgentsReturn {
  agents: RegistryAgent[];
  loading: boolean;
  error: string | null;
  hasMore: boolean;
  page: number;
  totalCount: number;
  fetchAgents: (reset?: boolean) => Promise<void>;
  searchAgents: (query: string) => Promise<void>;
  loadMore: () => void;
  refresh: () => Promise<void>;
}

const PAGE_SIZE = 20;

export function useRegistryAgents(): UseRegistryAgentsReturn {
  const [agents, setAgents] = useState<RegistryAgent[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [hasMore, setHasMore] = useState(false);
  const [page, setPage] = useState(1);
  const [totalCount, setTotalCount] = useState(0);
  const [searchQuery, setSearchQuery] = useState("");

  const fetchAgents = useCallback(
    async (reset = false) => {
      setLoading(true);
      setError(null);

      try {
        const currentPage = reset ? 1 : page;
        const result = await invoke<unknown>("get_registry_agents", {
          page: currentPage,
          pageSize: PAGE_SIZE,
          searchQuery: searchQuery || null,
          platformOnly: true,
        });

        const parsed = paginatedRegistryAgentsSchema.parse(result);

        if (reset) {
          setAgents(parsed.agents);
        } else {
          setAgents((prev) => [...prev, ...parsed.agents]);
        }

        setHasMore(parsed.hasMore);
        setPage(currentPage + 1);
        setTotalCount(parsed.totalCount);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        setLoading(false);
      }
    },
    [page, searchQuery]
  );

  const searchAgents = useCallback(
    async (query: string) => {
      setSearchQuery(query);
      setPage(1);
      await fetchAgents(true);
    },
    [fetchAgents]
  );

  const loadMore = useCallback(() => {
    if (!loading && hasMore) {
      fetchAgents(false);
    }
  }, [loading, hasMore, fetchAgents]);

  const refresh = useCallback(async () => {
    try {
      await invoke("refresh_registry_catalog");
      setPage(1);
      await fetchAgents(true);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [fetchAgents]);

  return {
    agents,
    loading,
    error,
    hasMore,
    page,
    totalCount,
    fetchAgents,
    searchAgents,
    loadMore,
    refresh,
  };
}
