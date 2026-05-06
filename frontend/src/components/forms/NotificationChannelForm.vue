<template>
  <div class="space-y-4">
    <div>
      <label class="label">Name</label>
      <input
        :value="modelValue.name"
        class="input"
        :placeholder="modelValue.type === 'telegram' ? 'My Telegram Channel' : 'My Email Channel'"
        required
        @input="updateField('name', inputValue($event))"
      />
    </div>

    <div>
      <label class="label">Type</label>
      <select :value="modelValue.type" class="input" @change="updateType(selectValue($event))">
        <option value="email">Email</option>
        <option value="telegram">Telegram</option>
      </select>
    </div>

    <!-- Email config -->
    <template v-if="modelValue.type === 'email'">
      <div class="grid grid-cols-2 gap-3">
        <div>
          <label class="label">SMTP Host</label>
          <input
            :value="modelValue.config.smtp_host"
            class="input"
            placeholder="smtp.gmail.com"
            @input="updateConfig('smtp_host', inputValue($event))"
          />
        </div>
        <div>
          <label class="label">SMTP Port</label>
          <input
            :value="modelValue.config.smtp_port"
            type="number"
            class="input"
            placeholder="587"
            @input="updateConfig('smtp_port', numberValue($event, 587))"
          />
        </div>
      </div>
      <div class="grid grid-cols-2 gap-3">
        <div>
          <label class="label">Username</label>
          <input :value="modelValue.config.smtp_user" class="input" @input="updateConfig('smtp_user', inputValue($event))" />
        </div>
        <div>
          <label class="label">Password</label>
          <input
            :value="modelValue.config.smtp_password_enc"
            type="password"
            class="input"
            @input="updateConfig('smtp_password_enc', inputValue($event))"
          />
        </div>
      </div>
      <div>
        <label class="label">From Address</label>
        <input
          :value="modelValue.config.smtp_from"
          class="input"
          placeholder="alerts@example.com"
          @input="updateConfig('smtp_from', inputValue($event))"
        />
      </div>
      <div>
        <label class="label">Recipients (comma-separated)</label>
        <input
          :value="modelValue.config.recipients_str"
          class="input"
          placeholder="ops@example.com, dev@example.com"
          @input="updateConfig('recipients_str', inputValue($event))"
        />
      </div>
      <ToggleSwitch
        :model-value="!!modelValue.config.use_tls"
        label="Use TLS"
        @update:model-value="updateConfig('use_tls', $event)"
      />
    </template>

    <!-- Telegram config -->
    <template v-else>
      <div>
        <label class="label">Bot Token</label>
        <input
          :value="modelValue.config.bot_token"
          class="input"
          placeholder="123456:ABC-DEF..."
          @input="updateConfig('bot_token', inputValue($event))"
        />
      </div>
      <div>
        <label class="label">Chat ID</label>
        <input
          :value="modelValue.config.chat_id"
          class="input"
          placeholder="-1001234567890"
          @input="updateConfig('chat_id', inputValue($event))"
        />
      </div>
      <ToggleSwitch
        :model-value="!!modelValue.config.disable_notification"
        label="Silent Notification (No sound)"
        @update:model-value="updateConfig('disable_notification', $event)"
      />
    </template>

    <ToggleSwitch
      :model-value="modelValue.is_enabled"
      label="Enabled"
      @update:model-value="updateField('is_enabled', $event)"
    />
  </div>
</template>

<script setup lang="ts">
import ToggleSwitch from '@/components/common/ToggleSwitch.vue'

type ChannelType = 'email' | 'telegram'

interface ConfigModel {
  smtp_host?: string
  smtp_port?: number
  smtp_user?: string
  smtp_password_enc?: string
  smtp_from?: string
  recipients_str?: string
  use_tls?: boolean
  bot_token?: string
  chat_id?: string
  disable_notification?: boolean
}

interface ChannelFormModel {
  name: string
  type: ChannelType
  is_enabled: boolean
  config: ConfigModel
}

const props = defineProps<{
  modelValue: ChannelFormModel
}>()

const emit = defineEmits<{
  'update:modelValue': [value: ChannelFormModel]
}>()

function inputValue(event: Event) {
  return (event.target as HTMLInputElement).value
}

function selectValue(event: Event): ChannelType {
  return (event.target as HTMLSelectElement).value as ChannelType
}

function checkedValue(event: Event) {
  return (event.target as HTMLInputElement).checked
}

function numberValue(event: Event, fallback: number) {
  const value = Number((event.target as HTMLInputElement).value)
  return Number.isFinite(value) ? value : fallback
}

function updateField<K extends keyof Omit<ChannelFormModel, 'config'>>(key: K, value: ChannelFormModel[K]) {
  emit('update:modelValue', { ...props.modelValue, config: { ...props.modelValue.config }, [key]: value })
}

function updateType(type: ChannelType) {
  updateField('type', type)
}

function updateConfig<K extends keyof ConfigModel>(key: K, value: ConfigModel[K]) {
  emit('update:modelValue', {
    ...props.modelValue,
    config: {
      ...props.modelValue.config,
      [key]: value,
    },
  })
}
</script>
