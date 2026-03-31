import { useCallback, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  type InstallationMethod,
  type InstallRuntimeResponse,
  installRuntimeResponseSchema,
} from "@/types/agent-runtime";
import {
  type RegistryAgent,
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
  installAgent: (agent: RegistryAgent) => Promise<InstallRuntimeResponse>;
  installingAgentId: string | null;
}

const PAGE_SIZE = 20;

export function useRegistryAgents(): UseRegistryAgentsReturn {
  const [agents, setAgents] = useState<RegistryAgent[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [hasMore, setHasMore] = useState(false);
  const [page, setPage] = useState(1);
  const [totalCount, setTotalCount] = useState(0);
  const [installingAgentId, setInstallingAgentId] = useState<string | null>(null);
  const pageRef = useRef(1);
  const searchQueryRef = useRef("");

  const fetchAgents = useCallback(
    async (reset = false) => {
      setLoading(true);
      setError(null);

      try {
        const currentPage = reset ? 1 : pageRef.current;
        const currentQuery = searchQueryRef.current;
        const result = await invoke<unknown>("get_registry_agents", {
          page: currentPage,
          pageSize: PAGE_SIZE,
          searchQuery: currentQuery || null,
          platformOnly: true,
        });

        const parsed = paginatedRegistryAgentsSchema.parse(result);

        if (reset) {
          setAgents(parsed.agents);
        } else {
          setAgents((prev) => [...prev, ...parsed.agents]);
        }

        setHasMore(parsed.hasMore);
        const nextPage = currentPage + 1;
        setPage(nextPage);
        pageRef.current = nextPage;
        setTotalCount(parsed.totalCount);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        setLoading(false);
      }
    },
    []
  );

  const searchAgents = useCallback(
    async (query: string) => {
      searchQueryRef.current = query;
      setPage(1);
      pageRef.current = 1;
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
      pageRef.current = 1;
      await fetchAgents(true);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [fetchAgents]);

  const installAgent = useCallback(
    async (agent: RegistryAgent) => {
      const method = (agent.preferredMethod === "binary" || agent.supportedMethods.includes("binary")
        ? "binary"
        : "npx") as InstallationMethod;

      setInstallingAgentId(agent.registryId);
      setError(null);

      try {
        const result = await invoke<unknown>("install_registry_agent", {
          registryId: agent.registryId,
          method,
        });
        const parsed = installRuntimeResponseSchema.parse(result);
        await fetchAgents(true);
        return parsed;
      } catch (e) {
        const message = e instanceof Error ? e.message : String(e);
        setError(message);
        throw e;
      } finally {
        setInstallingAgentId(null);
      }
    },
    [fetchAgents]
  );

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
    installAgent,
    installingAgentId,
  };
}
