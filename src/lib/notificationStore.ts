import { writable } from 'svelte/store';

export type NotificationType = 'error' | 'success' | 'warning' | 'info';

export interface Notification {
    id: number;
    message: string;
    type: NotificationType;
}

function createNotificationStore() {
    const { subscribe, update } = writable<Notification[]>([]);

    function send(message: string, type: NotificationType = 'info', duration: number = 3000) {
        const id = Date.now();
        update(n => [...n, { id, message, type }]);
        
        if (duration > 0) {
            setTimeout(() => {
                dismiss(id);
            }, duration);
        }
    }

    function dismiss(id: number) {
        update(n => n.filter(i => i.id !== id));
    }

    return {
        subscribe,
        send,
        error: (msg: string) => send(msg, 'error'),
        success: (msg: string) => send(msg, 'success'),
        warning: (msg: string) => send(msg, 'warning'),
        info: (msg: string) => send(msg, 'info'),
        dismiss
    };
}

export const notifications = createNotificationStore();
