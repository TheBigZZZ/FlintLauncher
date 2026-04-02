import { writable } from 'svelte/store';

interface DownloadState {
    installingVersion: string | null;
    installProgress: number;
    installStatus: string;
    downloadLogs: string[];
    error: string;
}

const initialState: DownloadState = {
    installingVersion: null,
    installProgress: 0,
    installStatus: '',
    downloadLogs: [],
    error: ''
};

export const downloadStore = writable<DownloadState>(initialState);

export function setInstallingVersion(version: string | null) {
    downloadStore.update(state => ({ ...state, installingVersion: version }));
}

export function setInstallProgress(progress: number) {
    downloadStore.update(state => ({ ...state, installProgress: progress }));
}

export function setInstallStatus(status: string) {
    downloadStore.update(state => ({ ...state, installStatus: status }));
}

export function addDownloadLog(log: string) {
    downloadStore.update(state => {
        const newLogs = [...state.downloadLogs, log];
        // Keep only last 300 logs to prevent memory bloat
        if (newLogs.length > 300) {
            newLogs.shift();
        }
        return {
            ...state,
            downloadLogs: newLogs
        };
    });
}

export function setDownloadLogs(logs: string[]) {
    downloadStore.update(state => ({ ...state, downloadLogs: logs }));
}

export function setError(error: string) {
    downloadStore.update(state => ({ ...state, error }));
}

export function clearDownloadState() {
    downloadStore.set(initialState);
}
