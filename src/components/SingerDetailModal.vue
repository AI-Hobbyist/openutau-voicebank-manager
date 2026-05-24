<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { marked } from 'marked'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { open as openBrowser } from '@tauri-apps/plugin-shell'
import { useMessage, useDialog } from 'naive-ui'
import { StoreData } from '../classes/store_data'

// 下载队列管理
interface DownloadTask {
  url: string
  id: string
  name?: string
  sha256?: string
  installSubdir?: string
}
const downloadQueue = ref<DownloadTask[]>([])
const currentTask = ref<DownloadTask | null>(null)

const showModal = ref(false)
const singerInfo = ref<any>(null)
const downloadProgress = ref(0)
const downloadStatus = ref<'downloading' | 'unzipping' | 'idle'>('idle')
const isDownloading = ref(false)
const vbStatuses = ref<Record<string, any>>({})
const downloadSpeed = ref('')
const downloadETA = ref('')
const message = useMessage()
const dialog = useDialog()
const emit = defineEmits(['update-status', 'queue-change'])

let unlistenProgress: UnlistenFn | null = null
let unlistenSpeed: UnlistenFn | null = null
let unlistenETA: UnlistenFn | null = null
let unlistenFinished: UnlistenFn | null = null
let unlistenError: UnlistenFn | null = null

const handleOpenLink = async (url: string) => {
  if (!url) return
  try {
    await openBrowser(url)
  } catch (err) {
    console.error('Failed to open link:', err)
  }
}

const startDownload = async (url: string, id: string, sha256?: string, installSubdir?: string) => {
  if (!url) {
    message.error('无效的下载链接')
    return
  }
  const settings = StoreData.loadSettings('app_settings') || {
    voicebankPath: './Singers',
    aria2Path: './3rd/aria2c.exe',
    sevenZipPath: './3rd/7-zip/x64/7za.exe',
    aria2X: 16,
    aria2S: 16
  }

  isDownloading.value = true
  downloadProgress.value = 0
  downloadStatus.value = 'downloading'

  try {
    await invoke('download_voicebank', {
      aria2Path: settings.aria2Path,
      sevenZipPath: settings.sevenZipPath || './3rd/7-zip/x64/7za.exe',
      url: url,
      savePath: settings.voicebankPath,
      threads: settings.aria2X,
      connections: settings.aria2S,
      installId: id,
      installSubdir: installSubdir,
      sha256: sha256
    })
    message.info(`正在下载: ${id}`)
  } catch (err: any) {
    message.error(`下载启动失败: ${err}`)
    processNextTask()
  }
}

const processNextTask = () => {
  if (downloadQueue.value.length === 0) {
    isDownloading.value = false
    currentTask.value = null
    downloadStatus.value = 'idle'
    return
  }
  
  const next = downloadQueue.value.shift()!
  currentTask.value = next
  emit('queue-change', {
    current: currentTask.value,
    queue: downloadQueue.value,
    progress: downloadProgress.value,
    status: downloadStatus.value,
    speed: downloadSpeed.value,
    eta: downloadETA.value
  })
  startDownload(next.url, next.id, next.sha256, next.installSubdir)
}

const updateStatuses = async () => {
  if (!singerInfo.value || !singerInfo.value.voicebanks) return

  const settings = StoreData.loadSettings('app_settings') || {
    voicebankPath: './Singers',
  }

  const vbs = Object.values(singerInfo.value.voicebanks)
  try {
    const statuses: any[] = await invoke('check_voicebank_status', {
      voicebankPath: settings.voicebankPath,
      voicebanks: vbs
    })
    const statusMap: Record<string, any> = {}
    statuses.forEach(s => {
      // 使用 id 和 install_subdir 作为复合键以确保唯一性
      const key = s.install_subdir ? `${s.id}|${s.install_subdir}` : s.id
      statusMap[key] = s
    })
    vbStatuses.value = statusMap
    emit('update-status', statuses)
  } catch (err) {
    console.error('Failed to check voicebank status:', err)
  }
}

const handleInstall = async (vb: any) => {
  if (!vb.url) {
    message.warning('未找到可安装的版本链接')
    return
  }

  const statusKey = vb.install_subdir ? `${vb.id}|${vb.install_subdir}` : vb.id

  if (isDownloading.value || downloadQueue.value.some(t => t.id === vb.id) || (currentTask.value?.id === vb.id)) {
    if (downloadQueue.value.some(t => t.id === vb.id) || currentTask.value?.id === vb.id) {
        message.warning('该任务已在队列中')
        return
    }
    downloadQueue.value.push({ url: vb.url, id: vb.id, name: singerInfo.value.name, sha256: vb.sha256, installSubdir: vb.install_subdir })
    emit('queue-change', { 
        current: currentTask.value, 
        queue: downloadQueue.value,
        progress: downloadProgress.value,
        status: downloadStatus.value,
        speed: downloadSpeed.value,
        eta: downloadETA.value
    })
    message.info('已加入下载队列')
    return
  }

  isDownloading.value = true
  currentTask.value = { url: vb.url, id: vb.id, name: singerInfo.value.name, sha256: vb.sha256, installSubdir: vb.install_subdir }
  emit('queue-change', {
    current: currentTask.value,
    queue: downloadQueue.value,
    progress: downloadProgress.value,
    status: downloadStatus.value,
    speed: downloadSpeed.value,
    eta: downloadETA.value
  })
  startDownload(vb.url, vb.id, vb.sha256, vb.install_subdir)
}

const getVbStatus = (vb: any) => {
  const key = vb.install_subdir ? `${vb.id}|${vb.install_subdir}` : vb.id
  return vbStatuses.value[key] || {}
}

const handleCancelDownload = async (vb: any) => {
  // 如果在队列中但还没开始下载
  const queueIndex = downloadQueue.value.findIndex(t => t.id === vb.id)
  if (queueIndex !== -1) {
    downloadQueue.value.splice(queueIndex, 1)
    emit('queue-change', {
        current: currentTask.value,
        queue: downloadQueue.value,
        progress: downloadProgress.value,
        status: downloadStatus.value,
        speed: downloadSpeed.value,
        eta: downloadETA.value
    })
    message.info('已从队列中取消')
    return
  }

  // 如果正在下载
  if (currentTask.value?.id === vb.id) {
    try {
      await invoke('cancel_download', { installId: vb.id })
      message.info('已取消下载')
      emit('queue-change', {
        current: currentTask.value,
        queue: downloadQueue.value,
        progress: downloadProgress.value,
        status: downloadStatus.value,
        speed: downloadSpeed.value,
        eta: downloadETA.value
      })
    } catch (err) {
      message.error(`取消失败: ${err}`)
    }
  }
}

const handleUninstall = (vb: any) => {
  dialog.error({
    title: '确认删除',
    content: `确定要删除声库 ${singerInfo.value.name}(id: ${vb.id}) 吗？该操作不可撤回。`,
    positiveText: '确认',
    negativeText: '取消',
    onPositiveClick: async () => {
      const settings = StoreData.loadSettings('app_settings') || {
        voicebankPath: './Singers',
      }
      
      if (!vb.install_subdir) {
        message.error('无法卸载：未指定安装子目录')
        return
      }

      try {
        await invoke('del_dir', { path: `${settings.voicebankPath}/${vb.install_subdir}` })
        message.success('卸载成功')
        updateStatuses()
        emit('update-status')
      } catch (err) {
        message.error(`卸载失败: ${err}`)
      }
    }
  })
}

watch(() => singerInfo.value, (newVal) => {
  if (newVal) {
    updateStatuses()
  }
}, { deep: true })

onMounted(async () => {
  unlistenProgress = await listen('download-progress', (event) => {
    const val = event.payload as number
    downloadProgress.value = val
    if (val > 98) {
      downloadStatus.value = 'unzipping'
    } else {
      downloadStatus.value = 'downloading'
    }
    emit('queue-change', {
      current: currentTask.value,
      queue: downloadQueue.value,
      progress: downloadProgress.value,
      status: downloadStatus.value,
      speed: downloadSpeed.value,
      eta: downloadETA.value
    })
  })

  unlistenSpeed = await listen('download-speed', (event) => {
    downloadSpeed.value = event.payload as string
    emit('queue-change', {
      current: currentTask.value,
      queue: downloadQueue.value,
      progress: downloadProgress.value,
      status: downloadStatus.value,
      speed: downloadSpeed.value,
      eta: downloadETA.value
    })
  })

  unlistenETA = await listen('download-eta', (event) => {
    downloadETA.value = event.payload as string
    emit('queue-change', {
      current: currentTask.value,
      queue: downloadQueue.value,
      progress: downloadProgress.value,
      status: downloadStatus.value,
      speed: downloadSpeed.value,
      eta: downloadETA.value
    })
  })

  unlistenFinished = await listen('download-finished', () => {
    message.success(`下载完成: ${currentTask.value?.id}`)
    downloadProgress.value = 100
    downloadStatus.value = 'idle'
    updateStatuses()
    emit('update-status')
    processNextTask()
    emit('queue-change', {
        current: currentTask.value,
        queue: downloadQueue.value,
        progress: 0,
        status: 'idle',
        speed: '',
        eta: ''
    })
  })

  unlistenError = await listen('download-error', (event) => {
    message.error(`下载出错: ${event.payload}`)
    downloadStatus.value = 'idle'
    processNextTask()
    emit('queue-change', {
        current: currentTask.value,
        queue: downloadQueue.value,
        progress: 0,
        status: 'idle',
        speed: '',
        eta: ''
    })
  })
})

onUnmounted(() => {
  if (unlistenProgress) unlistenProgress()
  if (unlistenSpeed) unlistenSpeed()
  if (unlistenETA) unlistenETA()
  if (unlistenFinished) unlistenFinished()
  if (unlistenError) unlistenError()
})

const openDetails = (data: any) => {
  singerInfo.value = data
  showModal.value = true
}

// 提取支持的语言列表 (优先取 top-level languages，否则从 voicebanks 中提取)
const supportedLanguages = computed(() => {
  if (!singerInfo.value) return []
  if (singerInfo.value.languages && singerInfo.value.languages.length > 0) {
    return singerInfo.value.languages
  }
  
  // 备选方案：从所有 voicebanks 中收集并去重
  const langs = new Set<string>()
  if (singerInfo.value.voicebanks) {
    Object.values(singerInfo.value.voicebanks).forEach((vb: any) => {
      if (vb.supportedLanguages) {
        vb.supportedLanguages.forEach((l: string) => langs.add(l))
      }
    })
  }
  return Array.from(langs)
})

const renderedLongDescription = computed(() => {
  if (!singerInfo.value?.long_description) return ''
  return marked.parse(singerInfo.value.long_description)
})

defineExpose({ openDetails, handleCancelDownload })
</script>

<template>
  <n-modal v-model:show="showModal">
    <div class="modal-content" v-if="singerInfo">
      <!-- 顶部 Header 区域 -->
      <div class="header-section">
        <div class="header-left">
          <n-avatar
            v-if="singerInfo.images && singerInfo.images.length > 0"
            :size="84"
            :src="singerInfo.images[0]"
            style="border-radius: 8px; border: 1px solid #444; object-fit: fill; background-color: #242424;"
          />
          <div class="header-info">
            <div class="singer-name">{{ singerInfo.name }}</div>
            <div class="lang-tags" v-if="supportedLanguages.length > 0">
              <n-tag
                v-for="lang in supportedLanguages.slice(0, 4)"
                :key="lang"
                round
                size="small"
                :bordered="false"
                class="lang-tag"
              >
                {{ lang }}
              </n-tag>
              <n-popover trigger="hover" v-if="supportedLanguages.length > 4">
                <template #trigger>
                  <n-tag
                    round
                    size="small"
                    :bordered="false"
                    class="lang-tag more-tag"
                    style="cursor: pointer;"
                  >
                    +{{ supportedLanguages.length - 4 }}
                  </n-tag>
                </template>
                <div class="popover-langs">
                  <n-tag
                    v-for="lang in supportedLanguages.slice(4)"
                    :key="lang"
                    round
                    size="small"
                    :bordered="false"
                    class="lang-tag"
                    style="margin: 2px;"
                  >
                    {{ lang }}
                  </n-tag>
                </div>
              </n-popover>
            </div>
            <div class="description-text" v-if="singerInfo.description">
              {{ singerInfo.description }}
            </div>
          </div>
        </div>
        <div class="header-right">
          <n-button v-if="singerInfo.website_url" secondary size="small" @click="handleOpenLink(singerInfo.website_url)">
            项目官网
          </n-button>
          <n-button v-if="singerInfo.link" secondary size="small" @click="handleOpenLink(singerInfo.link)">
            歌手主页
          </n-button>
        </div>
      </div>

      <!-- 说明文档 区域 -->
      <div class="notes-section" v-if="singerInfo.long_description || (singerInfo.images && singerInfo.images.length > 1)">
        <div class="section-title">说明文档</div>
        <div class="notes-content">
          <div class="extra-phonemes-container">
            <div class="phonemes-left" v-if="singerInfo.long_description">
              <div class="markdown-body" v-html="renderedLongDescription"></div>
            </div>
            <!-- 如果没有 long_description 但有 phonemes (兼容旧逻辑) -->
            <div class="phonemes-left" v-else-if="singerInfo.phonemes && singerInfo.phonemes.length > 0">
              <div class="sub-title">附加音素</div>
              <n-table :single-line="false" size="small" class="custom-table">
                <thead>
                  <tr>
                    <th style="width: 80px">音素</th>
                    <th style="width: 120px">名称</th>
                    <th>用法</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="p in singerInfo.phonemes" :key="p.code">
                    <td>{{ p.code }}</td>
                    <td>{{ p.name }}</td>
                    <td>{{ p.usage }}</td>
                  </tr>
                </tbody>
              </n-table>
            </div>
            <div class="chibi-container" v-if="singerInfo.images && singerInfo.images.length > 1">
              <img :src="singerInfo.images[1]" class="chibi-image" />
            </div>
          </div>
        </div>
      </div>

      <!-- Voicebank 区域 (循环展示不同引擎) -->
      <template v-if="singerInfo.voicebanks && Object.keys(singerInfo.voicebanks).length > 0">
        <div v-for="(vb, engine) in singerInfo.voicebanks" :key="engine" class="engine-section">
          <div class="engine-info">
            <div class="engine-name">{{ engine }}</div>
            <div class="engine-id" v-if="vb.id">{{ vb.id }}</div>
            <div class="engine-desc" v-if="vb.description">{{ vb.description }}</div>
            <n-space size="small" style="margin-top: 8px;">
              <!-- 未安装时的按钮 -->
              <template v-if="!getVbStatus(vb).installed && !(currentTask?.id === vb.id || downloadQueue.some(t => t.id === vb.id))">
                <n-popover trigger="hover" v-if="vb.sha256">
                  <template #trigger>
                    <n-button
                      size="small"
                      strong
                      @click="handleInstall(vb)"
                    >
                      安装
                    </n-button>
                  </template>
                  <div style="font-size: 12px; font-family: monospace;">
                    当前 SHA256:<br/>{{ vb.sha256 }}
                  </div>
                </n-popover>
                <n-button
                  v-else
                  size="small"
                  strong
                  @click="handleInstall(vb)"
                >
                  安装
                </n-button>
              </template>

              <!-- 下载/排队中的取消按钮 -->
              <n-button
                v-else-if="currentTask?.id === vb.id || downloadQueue.some(t => t.id === vb.id)"
                size="small"
                strong
                type="warning"
                @click="handleCancelDownload(vb)"
              >
                {{ (currentTask?.id === vb.id) ? '取消下载' : '取消排队' }}
              </n-button>
              
              <!-- 已安装时的逻辑 -->
              <template v-else>
                <!-- 更新按钮 (仅当有更新时显示) -->
                <n-popover trigger="hover" v-if="getVbStatus(vb).needs_update">
                  <template #trigger>
                    <n-button
                      size="small"
                      strong
                      type="warning"
                      @click="handleInstall(vb)"
                    >
                      更新
                    </n-button>
                  </template>
                  <div style="font-size: 12px; font-family: monospace;">
                    本地 SHA256:<br/>{{ getVbStatus(vb).local_sha256 || '未校验' }}
                  </div>
                </n-popover>

                <!-- 卸载按钮 -->
                <n-button
                  size="small"
                  strong
                  type="error"
                  @click="handleUninstall(vb)"
                >
                  卸载
                </n-button>
              </template>
              <n-button size="small" secondary @click="handleOpenLink(singerInfo.website_url)">官网</n-button>
            </n-space>
            <!-- 下载进度条 -->
            <div v-if="isDownloading && vb.url === (currentTask?.id === vb.id ? currentTask?.url : '')" style="margin-top: 8px;">
              <n-progress
                type="line"
                :percentage="downloadProgress"
                :indicator-placement="'inside'"
                processing
              />
            </div>
          </div>
        </div>
      </template>
    </div>
  </n-modal>
</template>

<style scoped>
.modal-content {
  width: 95vw;
  max-width: 1100px;
  background-color: #1a1a1a;
  border: 1px solid #333;
  border-radius: 8px;
  padding: 20px;
  color: #ccc;
  font-family: v-sans, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
}

/* Header */
.header-section {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 24px;
  flex-wrap: wrap;
  gap: 16px;
}

.header-left {
  display: flex;
  gap: 20px;
  flex: 1;
  min-width: 300px;
  flex-wrap: wrap;
}

.header-info {
  display: flex;
  flex-direction: column;
  gap: 8px;
  flex: 1;
  min-width: 200px;
}

.name-row {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}

.singer-name {
  font-size: 24px;
  font-weight: bold;
  color: #fff;
}

.lang-tags {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
}

.lang-tag {
  background-color: rgba(100, 200, 100, 0.1);
  color: #64c864;
  font-size: 10px;
  padding: 0 6px;
  height: 18px;
  border: 1px solid rgba(100, 200, 100, 0.3);
}

.description-text {
  font-size: 14px;
  color: #aaa;
}

.header-right {
  display: flex;
  gap: 8px;
}

/* Notes Section */
.notes-section {
  border: 1px solid #333;
  border-radius: 4px;
  margin-bottom: 20px;
  overflow: hidden;
}

.section-title {
  background-color: #2c2c2c;
  padding: 4px 12px;
  font-size: 12px;
  font-weight: bold;
  color: #888;
}

.notes-content {
  padding: 16px;
  background-color: #1e1e1e;
}

.team-info-title {
  font-size: 18px;
  font-weight: bold;
  color: #ddd;
  margin-bottom: 8px;
}

.sub-title {
  font-size: 14px;
  font-weight: bold;
  color: #ddd;
  margin-bottom: 12px;
}

.extra-phonemes-container {
  display: flex;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 16px;
}

.phonemes-left {
  flex: 1;
  min-width: 300px;
}

.custom-table {
  background-color: transparent;
}

:deep(.custom-table th) {
  background-color: #242424 !important;
  color: #888 !important;
  font-size: 12px;
  border-bottom: 1px solid #333 !important;
}

:deep(.custom-table td) {
  background-color: transparent !important;
  color: #ccc !important;
  font-size: 12px;
  border-bottom: 1px solid #333 !important;
}

.chibi-container {
  width: 180px;
  height: 180px;
  display: flex;
  justify-content: center;
  align-items: center;
}

.chibi-image {
  width: 180px; /* 固定正方形宽度 */
  height: 180px; /* 固定正方形高度 */
  object-fit: fill; /* 强制拉伸 */
}

/* Engine Section */
.engine-section {
  border: 1px solid #333;
  border-radius: 4px;
  padding: 16px;
  margin-bottom: 12px;
  background-color: #1e1e1e;
}

.engine-name {
  font-size: 16px;
  font-weight: bold;
  color: #fff;
}

.engine-id {
  font-size: 12px;
  color: #888;
}

.engine-desc {
  font-size: 12px;
  color: #aaa;
  margin-top: 4px;
}

/* Version Row */
.version-row {
  margin-top: 16px;
  background-color: #242424;
  border: 1px solid #333;
  border-radius: 4px;
  display: flex;
  align-items: center;
  padding: 8px 12px;
  flex-wrap: wrap;
  gap: 12px;
}

.version-left {
  display: flex;
  flex-direction: column;
  width: 100px;
}

.version-num {
  font-size: 12px;
  font-weight: bold;
  color: #fff;
}

.version-engine {
  font-size: 10px;
  color: #666;
}

.version-center {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 12px;
}

.release-tag {
  font-size: 12px;
  color: #888;
}

.latest-tag {
  background-color: rgba(100, 200, 100, 0.1) !important;
  color: #64c864 !important;
  font-size: 10px;
  border: 1px solid rgba(100, 200, 100, 0.2) !important;
}

.install-ver-btn {
  background-color: #333 !important;
  border: none !important;
  color: #ccc !important;
}

.install-ver-btn:hover {
  background-color: #444 !important;
}

.popover-langs {
  display: flex;
  flex-wrap: wrap;
  max-width: 250px;
  gap: 4px;
  padding: 4px;
}

/* Markdown 样式适配 */
.markdown-body {
  font-size: 14px;
  line-height: 1.6;
  color: #ccc;
}

:deep(.markdown-body h1),
:deep(.markdown-body h2),
:deep(.markdown-body h3) {
  color: #fff;
  margin-top: 16px;
  margin-bottom: 8px;
  font-weight: bold;
}

:deep(.markdown-body h1) { font-size: 1.5em; }
:deep(.markdown-body h2) { font-size: 1.3em; }
:deep(.markdown-body h3) { font-size: 1.1em; }

:deep(.markdown-body p) {
  margin-bottom: 8px;
}

:deep(.markdown-body table) {
  width: 100%;
  border-collapse: collapse;
  margin-bottom: 16px;
  background-color: transparent;
}

:deep(.markdown-body th) {
  background-color: #242424;
  color: #888;
  font-size: 12px;
  text-align: left;
  padding: 8px;
  border: 1px solid #333;
}

:deep(.markdown-body td) {
  padding: 8px;
  border: 1px solid #333;
  font-size: 12px;
  color: #ccc;
}

:deep(.markdown-body ul), 
:deep(.markdown-body ol) {
  padding-left: 20px;
  margin-bottom: 8px;
}
</style>
