<template>
  <div
    class="pointer-events-none fixed inset-x-3 bottom-3 z-50 flex flex-col gap-2 sm:inset-x-auto sm:right-4 sm:bottom-4 sm:w-96"
    aria-live="polite"
  >
    <TransitionGroup name="toast">
      <article
        v-for="toast in toastStore.toasts"
        :key="toast.id"
        class="toast pointer-events-auto relative overflow-hidden rounded-xl border shadow-lg"
        :class="tone(toast.type)"
        :role="toast.type === 'error' || toast.type === 'warning' ? 'alert' : 'status'"
        @mouseenter="pause(toast.id)"
        @mouseleave="resume(toast.id)"
      >
        <div class="flex items-start gap-3 px-4 py-3">
          <component :is="iconFor(toast.type)" class="mt-0.5 h-5 w-5 shrink-0" />
          <div class="min-w-0 flex-1">
            <p class="text-sm font-semibold">{{ toast.title }}</p>
            <p class="mt-0.5 text-sm leading-snug opacity-90">{{ toast.message }}</p>
          </div>
          <button class="rounded p-1 opacity-70 hover:opacity-100" type="button" aria-label="Dismiss" @click="dismiss(toast.id)">
            <IconX class="h-4 w-4" />
          </button>
        </div>
        <div class="h-1 w-full bg-black/10">
          <div class="toast-bar h-full" :style="barStyle(toast)" :class="{ paused: paused.has(toast.id) }" />
        </div>
      </article>
    </TransitionGroup>
  </div>
</template>

<script setup lang="ts">
import { onBeforeUnmount, reactive, watch } from 'vue'
import { IconAlertTriangle, IconCircleCheck, IconInfoCircle, IconX, IconXboxX } from '@tabler/icons-vue'
import { useToastStore, type Toast, type ToastType } from '@/stores/toast'

const toastStore = useToastStore()
const started = reactive(new Map<number, number>())
const remaining = reactive(new Map<number, number>())
const timers = new Map<number, ReturnType<typeof setTimeout>>()
const paused = reactive(new Set<number>())

watch(
  () => toastStore.toasts.map((toast) => toast.id),
  (ids) => {
    for (const id of ids) {
      if (!started.has(id)) arm(toastStore.toasts.find((toast) => toast.id === id)!)
    }
  },
)

function arm(toast: Toast) {
  started.set(toast.id, Date.now())
  remaining.set(toast.id, toast.timeout)
  timers.set(toast.id, setTimeout(() => dismiss(toast.id), toast.timeout))
}

function pause(id: number) {
  const timer = timers.get(id)
  if (!timer) return
  clearTimeout(timer)
  timers.delete(id)
  const elapsed = Date.now() - (started.get(id) ?? Date.now())
  remaining.set(id, Math.max(0, (remaining.get(id) ?? 0) - elapsed))
  paused.add(id)
}

function resume(id: number) {
  if (!paused.has(id)) return
  paused.delete(id)
  const left = remaining.get(id) ?? 0
  started.set(id, Date.now())
  remaining.set(id, left)
  timers.set(id, setTimeout(() => dismiss(id), left))
}

function dismiss(id: number) {
  const timer = timers.get(id)
  if (timer) clearTimeout(timer)
  timers.delete(id)
  started.delete(id)
  remaining.delete(id)
  paused.delete(id)
  toastStore.remove(id)
}

function iconFor(type: ToastType) {
  if (type === 'success') return IconCircleCheck
  if (type === 'error') return IconXboxX
  if (type === 'warning') return IconAlertTriangle
  return IconInfoCircle
}

function tone(type: ToastType) {
  if (type === 'success') return 'border-mint/40 bg-surface-800 text-mint'
  if (type === 'error') return 'border-brand-500/50 bg-surface-800 text-brand-400'
  if (type === 'warning') return 'border-cream/40 bg-surface-800 text-cream'
  return 'border-mist/40 bg-surface-800 text-mist'
}

function barStyle(toast: Toast) {
  return { animationDuration: `${toast.timeout}ms` }
}

onBeforeUnmount(() => {
  for (const timer of timers.values()) clearTimeout(timer)
})
</script>

<style scoped>
.toast-enter-active, .toast-leave-active { transition: all 0.25s ease; }
.toast-enter-from, .toast-leave-to { opacity: 0; transform: translateY(10px); }
.toast-bar { background: currentColor; transform-origin: left; animation-name: shrink; animation-timing-function: linear; animation-fill-mode: forwards; }
.toast-bar.paused { animation-play-state: paused; }
@keyframes shrink { from { transform: scaleX(1); } to { transform: scaleX(0); } }
</style>
