<template>
  <span v-if="enabled" class="inline-flex shrink-0" :title="label">
    <IconCertificate class="h-4 w-4" :style="{ color }" :aria-label="label" />
  </span>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { IconCertificate } from '@tabler/icons-vue'

const props = defineProps<{
  enabled: boolean
  valid: boolean | null
  days: number | null
  warnDays: number
}>()

const tone = computed(() => {
  if (props.valid == null && props.days == null) return 'wait' as const
  if (props.valid === false) return 'hot' as const
  if (props.days == null) return 'good' as const
  if (props.days > props.warnDays) return 'good' as const
  if (props.days > props.warnDays / 2) return 'warm' as const
  return 'hot' as const
})

const color = computed(() => {
  if (tone.value === 'good') return '#B4D7CD'
  if (tone.value === 'warm') return '#FFEEB3'
  if (tone.value === 'hot') return '#F47321'
  return '#8EA8C3'
})

const label = computed(() => {
  if (tone.value === 'wait') return 'SSL check has not run'
  if (props.valid === false) return 'Certificate is not valid'
  if (props.days == null) return 'Certificate is valid'
  if (tone.value === 'good') return `Certificate healthy, ${props.days} days left`
  return `Certificate expires in ${props.days} days`
})
</script>
