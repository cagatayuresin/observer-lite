import { defineStore } from 'pinia'
import { ref } from 'vue'

export type ToastType = 'success' | 'error' | 'warning' | 'info'

export interface Toast {
  id: number
  type: ToastType
  title: string
  message: string
  timeout: number
}

const DEFAULT_TIMEOUT: Record<ToastType, number> = {
  success: 4000,
  info: 5000,
  warning: 7000,
  error: 9000,
}

const DEFAULT_TITLE: Record<ToastType, string> = {
  success: 'Saved',
  info: 'Notice',
  warning: 'Check this',
  error: 'Something failed',
}

let nextId = 0

export const useToastStore = defineStore('toast', () => {
  const toasts = ref<Toast[]>([])

  function push(type: ToastType, message: string, title?: string, timeout?: number) {
    const id = ++nextId
    toasts.value.push({
      id,
      type,
      title: title || DEFAULT_TITLE[type],
      message,
      timeout: timeout ?? DEFAULT_TIMEOUT[type],
    })
    return id
  }

  function remove(id: number) {
    const index = toasts.value.findIndex((toast) => toast.id === id)
    if (index !== -1) toasts.value.splice(index, 1)
  }

  const success = (message: string, title?: string) => push('success', message, title)
  const error = (message: string, title?: string) => push('error', message, title)
  const warning = (message: string, title?: string) => push('warning', message, title)
  const info = (message: string, title?: string) => push('info', message, title)

  return { toasts, push, remove, success, error, warning, info }
})
