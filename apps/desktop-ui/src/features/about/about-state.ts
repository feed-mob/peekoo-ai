export interface UpdateLike {
  version: string;
  date?: string;
  body?: string;
}

export interface AboutSnapshot {
  appName: string;
  currentVersion: string;
  availableVersion: string | null;
  releaseDate: string | null;
  releaseNotes: string | null;
  isUpdateAvailable: boolean;
}

interface AboutDependencies {
  getName: () => Promise<string>;
  getVersion: () => Promise<string>;
  check: () => Promise<UpdateLike | null>;
}

export interface LoadAboutSnapshotResult {
  snapshot: AboutSnapshot;
  update: UpdateLike | null;
  updateError: string | null;
}

function getErrorMessage(error: unknown): string {
  if (error instanceof Error && error.message) {
    return error.message;
  }

  return "Unknown error";
}

export async function loadAboutSnapshot(deps: AboutDependencies): Promise<LoadAboutSnapshotResult> {
  const [appNameResult, currentVersionResult, updateResult] = await Promise.allSettled([
    deps.getName(),
    deps.getVersion(),
    deps.check(),
  ]);

  if (appNameResult.status === "rejected") {
    throw appNameResult.reason;
  }

  if (currentVersionResult.status === "rejected") {
    throw currentVersionResult.reason;
  }

  const update = updateResult.status === "fulfilled" ? updateResult.value : null;
  const updateError = updateResult.status === "rejected" ? getErrorMessage(updateResult.reason) : null;

  return {
    snapshot: {
      appName: appNameResult.value,
      currentVersion: currentVersionResult.value,
      availableVersion: update?.version ?? null,
      releaseDate: update?.date ?? null,
      releaseNotes: update?.body ?? null,
      isUpdateAvailable: update !== null,
    },
    update,
    updateError,
  };
}
