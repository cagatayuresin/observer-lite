<template>
  <Teleport to="body">
    <div v-if="modelValue" class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60">
      <div class="card w-full max-w-md p-6">
        <h3 class="text-lg font-semibold text-white mb-2">{{ title }}</h3>
        <p class="text-slate-400 text-sm mb-6">{{ message }}</p>
        <div class="flex gap-3 justify-end">
          <button class="btn-ghost" @click="emit('update:modelValue', false)">Cancel</button>
          <button :class="danger ? 'btn-danger' : 'btn-primary'" @click="emit('confirm'); emit('update:modelValue', false)">
            {{ confirmLabel }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
defineProps<{ modelValue: boolean; title: string; message: string; confirmLabel?: string; danger?: boolean }>()
const emit = defineEmits<{ 'update:modelValue': [boolean]; confirm: [] }>()
</script>
