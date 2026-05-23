<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open as openBrowser } from '@tauri-apps/plugin-shell'
import {
  darkTheme,
  NIcon,
} from 'naive-ui'

import SingerDetailModal from './SingerDetailModal.vue'
import Settings from './Settings.vue'
import { StoreData } from '../classes/store_data'
import { getRequest } from '../classes/requests'

const detailModalRef = ref<InstanceType<typeof SingerDetailModal> | null>(null)
const settingsRef = ref<InstanceType<typeof Settings> | null>(null)
const loading = ref(false)
const searchQuery = ref('')
const downloadProgress = ref(0)

const handleOpenInstallDir = async () => {
  const savedSettings = StoreData.loadSettings('app_settings')
  const path = savedSettings?.voicebankPath || './Singers'
  try {
    await invoke('open_dir', { path })
  } catch (err) {
    console.error('Failed to open directory:', err)
  }
}

const handleOpen = (item: any) => {
  // 确保此时 item 包含 avatar, languages, phonemes 等字段 [2][3]
  detailModalRef.value?.openDetails(item)
}

const handleOpenSettings = () => {
  settingsRef.value?.openSettings()
}

const handleOpenLink = async (url: string) => {
  if (!url) return
  try {
    await openBrowser(url)
  } catch (err) {
    console.error('Failed to open link:', err)
  }
}

// 响应式数据
const rawData = ref<any>({
  registry: "",
  singers: {}
})

const installedCount = ref(0)
const pendingUpdateCount = ref(0)
const updateMap = ref<Record<string, string[]>>({}) // singerName -> [vbId1, vbId2]
const installedMap = ref<Record<string, string[]>>({}) // singerName -> [vbId1, vbId2]

const updateOverallStatus = async (specificStatuses?: any[]) => {
  if (!rawData.value.singers) return

  const settings = StoreData.loadSettings('app_settings') || {
    voicebankPath: './Singers',
  }

  try {
    let statuses: any[] = []
    if (specificStatuses) {
        // 如果是从详情页同步过来的，尝试合并而不是全量刷新 (这里简单处理直接用参数)
        // 但通常为了准确，全量查一次也没关系，如果 specificStatuses 只是部分的
        // 这里采用最稳妥的全量拉取
    }

    // 收集所有 voicebanks
    const allVbs: any[] = []
    Object.values(rawData.value.singers).forEach((singer: any) => {
        if (singer.voicebanks) {
            Object.values(singer.voicebanks).forEach((vb: any) => {
                allVbs.push(vb)
            })
        }
    })

    statuses = await invoke('check_voicebank_status', {
      voicebankPath: settings.voicebankPath,
      voicebanks: allVbs
    })

    installedCount.value = statuses.filter(s => s.installed).length
    pendingUpdateCount.value = statuses.filter(s => s.needs_update).length

    // 更新映射
    const newInstalledMap: Record<string, string[]> = {}
    const newUpdateMap: Record<string, string[]> = {}
    
    Object.values(rawData.value.singers).forEach((singer: any) => {
      if (singer.voicebanks) {
        const singerVbs = Object.values(singer.voicebanks)
        
        // 已安装
        const installedIds = singerVbs
          .filter((vb: any) => statuses.find(s => s.id === vb.id && s.install_subdir === vb.install_subdir && s.installed))
          .map((vb: any) => vb.id)
        if (installedIds.length > 0) {
          newInstalledMap[singer.name] = installedIds
        }

        // 有更新
        const updateIds = singerVbs
          .filter((vb: any) => statuses.find(s => s.id === vb.id && s.install_subdir === vb.install_subdir && s.needs_update))
          .map((vb: any) => vb.id)
        if (updateIds.length > 0) {
            newUpdateMap[singer.name] = updateIds
        }
      }
    })
    installedMap.value = newInstalledMap
    updateMap.value = newUpdateMap
  } catch (err) {
    console.error('Failed to check overall voicebank status:', err)
  }
}

const fetchData = async () => {
  loading.value = true
  const savedSettings = StoreData.loadSettings('app_settings')
  const apiUrl = savedSettings?.dataSource || 'https://res.ai-lab.top/api/voicebanks.json'
  
  try {
    const response = await getRequest(apiUrl)
    if (response.statusCode === 200) {
      rawData.value = response.result
      await updateOverallStatus()
    } else {
      console.error('Failed to fetch voicebanks:', response.result)
    }
  } finally {
    loading.value = false
  }
}

const showQueueDrawer = ref(false)
const globalCurrentTask = ref<any>(null)
const globalDownloadQueue = ref<any[]>([])

const globalDownloadStatus = ref('idle')
const globalDownloadSpeed = ref('')
const globalDownloadETA = ref('')

const handleQueueChange = (data: { current: any, queue: any[], progress: number, status: string, speed: string, eta?: string }) => {
  globalCurrentTask.value = data.current
  globalDownloadQueue.value = [...data.queue]
  downloadProgress.value = data.progress
  globalDownloadStatus.value = data.status
  globalDownloadSpeed.value = data.speed || ''
  globalDownloadETA.value = data.eta || ''
}

const totalTasks = ref(0)
const finishedTasks = ref(0)

watch(() => globalCurrentTask.value, (newVal, oldVal) => {
    if (newVal && !oldVal && globalDownloadQueue.value.length >= 0) {
        // 开启了新一轮下载
        totalTasks.value = globalDownloadQueue.value.length + 1
        finishedTasks.value = 0
    }
}, { immediate: true })

const overallTotalProgress = computed(() => {
    if (totalTasks.value === 0) return 0
    const eachWeight = 100 / totalTasks.value
    // 当前任务的完成度 + 已完成任务的权重
    const currentWeight = (downloadProgress.value / 100) * eachWeight
    return Math.round((finishedTasks.value * eachWeight) + currentWeight)
})

// 在 queue-change 里更新 finishedTasks 有点难，因为它是快照。
// 我们可以通过监听 download-finished 事件来增加 finishedTasks。

const handleCancelInQueue = (id: string) => {
  detailModalRef.value?.handleCancelDownload({ id })
}

onMounted(async () => {
  fetchData()

  // 这里不再监听下载事件，全部通过 SingerDetailModal 的 queue-change 事件同步
})

// 按 team 分组数据并应用搜索过滤
const groupedSingers = computed(() => {
  const groups: Record<string, any[]> = {}
  if (!rawData.value.singers) return groups

  const query = searchQuery.value.toLowerCase().trim()

  Object.values(rawData.value.singers).forEach((singer: any) => {
    const matchesQuery = !query ||
      singer.name?.toLowerCase().includes(query) ||
      singer.description?.toLowerCase().includes(query) ||
      singer.team?.toLowerCase().includes(query)

    if (matchesQuery) {
      if (!groups[singer.team]) groups[singer.team] = []
      groups[singer.team].push(singer)
    }
  })
  return groups
})

const singerCount = computed(() => Object.keys(rawData.value.singers || {}).length)
const filteredCount = computed(() => {
  return Object.values(groupedSingers.value).reduce((acc, curr) => acc + curr.length, 0)
})

const getLanguages = (singer: any) => {
  if (!singer.voicebanks) return 'default'
  const vbs = Object.values(singer.voicebanks)
  for (const vb of vbs as any[]) {
    if (vb.supportedLanguages && vb.supportedLanguages.length > 0) {
      return vb.supportedLanguages.slice(0, 5).join(', ') + '...'
    }
  }
  return 'default'
}
</script>

<template>
  <!-- 使用深色主题 -->
  <n-config-provider :theme="darkTheme">
    <n-layout position="absolute" style="background-color: #1a1a1a;">

      <!-- 顶部状态栏 -->
      <n-layout-header
        dir="ltr"
        style="
          background: rgba(26, 26, 26, 0.8);
          backdrop-filter: blur(8px);
          padding: 20px 24px;
          position: sticky;
          top: 0;
          z-index: 100;
          border-bottom: 1px solid #333;
        "
      >
        <n-flex justify="space-between" align="center">
          <div>
            <n-h2 style="margin: 0; margin-left: -130px;">声库管理</n-h2>
            <n-flex style="margin-top: 8px;" :size="[8, 8]">
              <n-tag :bordered="false" type="success" size="small" round>已安装 {{ installedCount }}</n-tag>
              <n-tag :bordered="false" type="warning" size="small" round>待更新 {{ pendingUpdateCount }}</n-tag>
              <n-tag :bordered="false" type="info" size="small" round>声库总量 {{ singerCount }}</n-tag>
              <n-tag v-if="searchQuery" :bordered="false" type="primary" size="small" round>已过滤 {{ filteredCount }}</n-tag>
            </n-flex>
          </div>
          <n-flex :size="[12, 8]" align="center">
            <n-input
              v-model:value="searchQuery"
              placeholder="搜索名称、描述或团队..."
              clearable
              style="width: 240px"
            >
              <template #prefix>
                <n-icon>
                  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512">
                    <path d="M456.69 421.39L362.6 327.3a173.81 173.81 0 0034.84-104.58C397.44 126.38 319.06 48 222.72 48S48 126.38 48 222.72s78.38 174.72 174.72 174.72A173.81 173.81 0 00327.3 362.6l94.09 94.09a25 25 0 0035.3-35.3zM98 222.72a124.72 124.72 0 11124.72 124.72A124.72 124.72 0 0198 222.72z" fill="currentColor"/>
                  </svg>
                </n-icon>
              </template>
            </n-input>
            <n-button secondary @click="showQueueDrawer = true">
              <template #icon>
                <n-badge :value="globalDownloadQueue.length" :show="globalDownloadQueue.length > 0">
                  <n-icon>
                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512">
                      <path d="M416 128V48c0-26.51-21.49-48-48-48H144c-26.51 0-48 21.49-48 48v80c-26.51 0-48 21.49-48 48v288c0 26.51 21.49 48 48 48h224c26.51 0 48-21.49 48-48V176c0-26.51-21.49-48-48-48zM160 64h192v64H160V64zm224 400H128V192h256v272z" fill="currentColor"/>
                    </svg>
                  </n-icon>
                </n-badge>
              </template>
              队列
            </n-button>
            <n-button secondary @click="handleOpenInstallDir">打开声库目录</n-button>
            <n-button secondary @click="handleOpenSettings">设置</n-button>
            <n-button secondary @click="fetchData" :loading="loading">刷新列表</n-button>
          </n-flex>
        </n-flex>
      </n-layout-header>

      <!-- 歌手分组列表 -->
      <div style="padding: 24px;">
      <div v-for="(singers, team) in groupedSingers" :key="team" style="margin-bottom: 40px;">
        <n-h3 prefix="bar" align-text>{{ team }}</n-h3>

        <n-flex x-gap="16" y-gap="16" justify="center">
          <div v-for="singer in singers" :key="singer.name" class="card-wrapper" @click="handleOpen(singer)">
            <n-card bordered class="singer-card" hoverable>
              <!-- 歌手图片 [6] -->
              <template #cover>
                <div class="image-container">
                  <n-image width="100%" :src="singer.images?.[0]" preview-disabled style="border-radius: 8px 8px 0 0;" />
                  <!-- 已安装角标 -->
                  <n-popover trigger="hover" v-if="installedMap[singer.name]">
                    <template #trigger>
                      <div class="installed-badge">
                        <n-icon size="16" color="#fff">
                          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512">
                            <path d="M186.3 332.1l-65.4-65.4c-6.2-6.2-16.4-6.2-22.6 0s-6.2 16.4 0 22.6l76.7 76.7c6.2 6.2 16.4 6.2 22.6 0l174.2-174.2c6.2-6.2 6.2-16.4 0-22.6s-16.4-6.2-22.6 0L186.3 332.1z" fill="currentColor"/>
                          </svg>
                        </n-icon>
                      </div>
                    </template>
                    <div style="font-size: 12px;">
                      已安装版本：<br/>
                      <div v-for="id in installedMap[singer.name]" :key="id" style="color: #64c864;">
                        • {{ id }}
                      </div>
                    </div>
                  </n-popover>

                  <!-- 待更新角标 -->
                  <n-popover trigger="hover" v-if="updateMap[singer.name]">
                    <template #trigger>
                      <div class="update-badge">
                        <n-icon size="12" color="#fff">
                            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512">
                                <path d="M256 80c0-8.8-7.2-16-16-16s-16 7.2-16 16v144c0 8.8 7.2 16 16 16h112c8.8 0 16-7.2 16-16s-7.2-16-16-16h-96V80zM256 0C114.6 0 0 114.6 0 256s114.6 256 256 256s256-114.6 256-256S397.4 0 256 0zM256 464c-114.7 0-208-93.3-208-208S141.3 48 256 48s208 93.3 208 208s-93.3 208-208 208z" fill="currentColor"/>
                                <circle cx="256" cy="272" r="32" fill="currentColor"/>
                                <path d="M256 202c-11 0-20 9-20 20v110c0 11 9 20 20 20s20-9 20-20V222c0-11-9-20-20-20zM256 354c-11 0-20 9-20 20s9 20 20 20 20-9 20-20-9-20-20-20z" fill="currentColor"/>
                            </svg>
                        </n-icon>
                      </div>
                    </template>
                    <div style="font-size: 12px;">
                      发现新版本：<br/>
                      <div v-for="id in updateMap[singer.name]" :key="id" style="color: #f0a020;">
                        • {{ id }}
                      </div>
                    </div>
                  </n-popover>
                </div>
              </template>

              <!-- 歌手信息 -->
              <n-space vertical size="small">
                <n-text strong style="font-size: 1.2rem;">{{ singer.name }}</n-text>
                <n-text depth="3" style="font-size: 0.8rem;">
                  {{ Object.keys(singer.voicebanks || {}).length }} 个版本<br />
                  {{ getLanguages(singer) }}
                </n-text>
                <n-text depth="2" class="description">
                  {{ singer.description }}
                </n-text>
              </n-space>

              <!-- 操作按钮 [4] -->
              <template #action>
                <n-flex size="small" justify="center" @click.stop>
                  <n-button @click="handleOpen(singer)" size="small" secondary>详情</n-button>
                  <n-button size="small" secondary @click="handleOpenLink(singer.website_url)">
                    官网
                  </n-button>
                  <n-button size="small" secondary @click="handleOpenLink(singer.link)">
                    主页
                  </n-button>
                </n-flex>
              </template>
            </n-card>
          </div>
        </n-flex>
      </div>

      <n-empty
        v-if="Object.keys(groupedSingers).length === 0"
        description="找不着对应的声库，换个关键词试试？"
        style="margin-top: 100px;"
      />
      </div>
      <SingerDetailModal
        ref="detailModalRef"
        @update-status="updateOverallStatus"
        @queue-change="handleQueueChange"
      />
      <Settings ref="settingsRef" />

      <!-- 下载队列侧栏 -->
      <n-drawer v-model:show="showQueueDrawer" :width="350" placement="right">
        <n-drawer-content title="下载队列" closable scrollable>
          <div v-if="!globalCurrentTask && globalDownloadQueue.length === 0" style="text-align: center; color: #666; margin-top: 40px;">
            队列中没有任务
          </div>
          
          <n-space vertical size="large" v-else>
            <!-- 当前下载 -->
            <div v-if="globalCurrentTask">
              <n-text depth="3" style="font-size: 12px; margin-bottom: 8px; display: block;">正在下载</n-text>
              <n-card size="small" bordered>
                <n-flex justify="space-between" align="center">
                  <div style="flex: 1; min-width: 0;">
                    <n-text strong style="display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                      {{ globalCurrentTask.name }}
                    </n-text>
                    <n-text depth="3" style="font-size: 11px;">ID: {{ globalCurrentTask.id }}</n-text>
                  </div>
                  <n-button circle size="small" type="warning" @click="handleCancelInQueue(globalCurrentTask.id)">
                    <template #icon>
                      <n-icon>
                        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512">
                          <path d="M405 136.79L375.21 107 256 226.21 136.79 107 107 136.79 226.21 256 107 375.21 136.79 405 256 285.79 375.21 405 405 375.21 285.79 256 405 136.79z" fill="currentColor"/>
                        </svg>
                      </n-icon>
                    </template>
                  </n-button>
                </n-flex>
                <div style="margin-top: 8px;">
                  <n-flex vertical size="small">
                    <n-flex justify="space-between" align="center">
                        <n-text depth="3" style="font-size: 11px;">
                        {{ globalDownloadStatus === 'unzipping' ? '正在解压...' : `正在下载... ${downloadProgress}%` }}
                        </n-text>
                        <n-flex size="small">
                            <n-text v-if="globalDownloadStatus === 'downloading' && globalDownloadETA" depth="3" style="font-size: 11px;">
                                剩余 {{ globalDownloadETA }}
                            </n-text>
                            <n-text v-if="globalDownloadStatus === 'downloading'" depth="3" style="font-size: 11px; color: #18a058;">
                                {{ globalDownloadSpeed }}
                            </n-text>
                        </n-flex>
                    </n-flex>
                    <n-progress
                      type="line"
                      :percentage="downloadProgress"
                      :processing="globalDownloadStatus !== 'idle'"
                      :status="globalDownloadStatus === 'unzipping' ? 'info' : 'success'"
                      size="small"
                    />
                  </n-flex>
                </div>
              </n-card>
            </div>

            <!-- 排队中 -->
            <div v-if="globalDownloadQueue.length > 0">
              <n-text depth="3" style="font-size: 12px; margin-bottom: 8px; display: block;">等待中 ({{ globalDownloadQueue.length }})</n-text>
              <n-space vertical size="small">
                <n-card v-for="task in globalDownloadQueue" :key="task.id" size="small" bordered>
                  <n-flex justify="space-between" align="center">
                    <div style="flex: 1; min-width: 0;">
                      <n-text strong style="display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                        {{ task.name }}
                      </n-text>
                      <n-text depth="3" style="font-size: 11px;">ID: {{ task.id }}</n-text>
                    </div>
                    <n-button circle size="small" secondary @click="handleCancelInQueue(task.id)">
                      <template #icon>
                        <n-icon>
                          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512">
                            <path d="M405 136.79L375.21 107 256 226.21 136.79 107 107 136.79 226.21 256 107 375.21 136.79 405 256 285.79 375.21 405 405 375.21 285.79 256 405 136.79z" fill="currentColor"/>
                          </svg>
                        </n-icon>
                      </template>
                    </n-button>
                  </n-flex>
                </n-card>
              </n-space>
            </div>
          </n-space>

          <template #footer v-if="globalCurrentTask">
            <div style="width: 100%;">
                <n-divider style="margin: 12px 0;" />
                <n-flex vertical size="small">
                    <n-flex justify="space-between">
                        <n-text depth="3" style="font-size: 12px;">总体进度</n-text>
                        <n-text depth="3" style="font-size: 12px;">{{ globalDownloadQueue.length }} 个任务待处理</n-text>
                    </n-flex>
                    <n-progress
                      type="line"
                      :percentage="overallTotalProgress"
                      processing
                      status="success"
                    />
                </n-flex>
            </div>
          </template>
        </n-drawer-content>
      </n-drawer>
    </n-layout>
  </n-config-provider>
</template>

<style scoped>
.singer-card {
  background-color: #2c2c2e;
  border-radius: 12px;
  height: 100%;
  transition: all 0.3s ease;
  border: 1px solid transparent;
}

.card-wrapper {
  width: 300px;
  cursor: pointer;
}

.card-wrapper:hover .singer-card {
  transform: translateY(-8px);
  border-color: #18a058;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
}

.image-container {
  padding: 12px;
  background: #242424;
  height: 260px;
  display: flex;
  justify-content: center;
  align-items: center;
  overflow: hidden;
}

.image-container :deep(img) {
  width: 260px; /* 固定宽度 */
  height: 260px; /* 固定高度，形成正方形 */
  object-fit: fill; /* 强制拉伸 */
}

.installed-badge {
  position: absolute;
  top: 8px;
  left: 8px;
  background-color: #18a058;
  width: 24px;
  height: 24px;
  border-radius: 50%;
  display: flex;
  justify-content: center;
  align-items: center;
  box-shadow: 0 2px 8px rgba(0,0,0,0.4);
  z-index: 10;
}

.update-badge {
  position: absolute;
  top: 8px;
  right: 8px;
  background-color: #f0a020;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  display: flex;
  justify-content: center;
  align-items: center;
  box-shadow: 0 2px 8px rgba(0,0,0,0.4);
  z-index: 10;
}

.description {
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
  font-size: 0.85rem;
  line-height: 1.4;
}

:deep(.n-card-header) {
  padding-bottom: 0;
}

:deep(.n-input .n-input__input-el) {
  text-align: left;
}
</style>