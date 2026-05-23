<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { StoreData } from '../classes/store_data'

const showModal = ref(false)

const settings = ref({
  voicebankPath: 'Singers',
  dataSource: 'https://res.ai-lab.top/api/voicebanks.json',
  aria2Path: './3rd/aria2c.exe',
  sevenZipPath: './3rd/7-zip/x64/7za.exe',
  aria2X: 16,
  aria2S: 16
})

const SETTINGS_KEY = 'app_settings'

const loadSettings = () => {
  const saved = StoreData.loadSettings(SETTINGS_KEY)
  if (saved) {
    settings.value = { ...settings.value, ...saved }
  }
}

const saveSettings = () => {
  StoreData.saveSettings(SETTINGS_KEY, settings.value)
}

onMounted(() => {
  loadSettings()
})

// 监听设置变化并保存，或者提供保存按钮
watch(settings, () => {
  saveSettings()
}, { deep: true })

const openSettings = () => {
  showModal.value = true
}

defineExpose({ openSettings })
</script>

<template>
  <n-modal v-model:show="showModal" preset="card" title="设置" style="width: 500px">
    <n-form label-placement="left" label-width="120" :model="settings">
      <n-divider>基础设置</n-divider>
      <n-form-item label="声库路径">
        <n-input v-model:value="settings.voicebankPath" placeholder="默认 Singers" />
      </n-form-item>
      <n-form-item label="数据源 URL">
        <n-input v-model:value="settings.dataSource" placeholder="例如 https://res.ai-lab.top/api/voicebanks.json" />
      </n-form-item>
      <n-divider>Aria2 设置</n-divider>
      <n-form-item label="Aria2 路径">
        <n-input v-model:value="settings.aria2Path" placeholder="默认 ./3rd/aria2c.exe" />
      </n-form-item>
      <n-grid :cols="2" :x-gap="12">
        <n-form-item-gi label="分段数 (-x)">
          <n-input-number v-model:value="settings.aria2X" :min="1" :max="128" style="width: 100%" />
        </n-form-item-gi>
        <n-form-item-gi label="连接数 (-s)">
          <n-input-number v-model:value="settings.aria2S" :min="1" :max="16" style="width: 100%" />
        </n-form-item-gi>
      </n-grid>
      <n-divider>解压设置</n-divider>
      <n-form-item label="7-Zip 路径">
        <n-input v-model:value="settings.sevenZipPath" placeholder="默认 ./3rd/7-zip/x64/7za.exe" />
      </n-form-item>
    </n-form>
    <template #footer>
      <n-flex justify="end">
        <n-button @click="showModal = false">关闭</n-button>
      </n-flex>
    </template>
  </n-modal>
</template>

<style scoped>
</style>
