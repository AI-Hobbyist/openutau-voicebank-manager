<script setup lang="ts">
import Main from './components/Main.vue'
</script>

<template>
  <n-dialog-provider>
    <n-message-provider>
      <Main />
    </n-message-provider>
  </n-dialog-provider>
</template>
