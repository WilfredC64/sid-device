<!--
  Copyright (C) 2022 Wilfred Bos
  Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.
-->

<template>
    <AboutDialog/>
</template>

<script>

import AboutDialog from '@/components/AboutDialog.vue'
import tauri from '@/utils/tauri';

export default {
  name: 'AboutPage',
  setup() {
    tauri.disableContextMenu();
    tauri.disableDefaultKeys();
    return {}
  },
  components: {
    AboutDialog
  }
}

</script>

<style>

    #about {
      font-family: Arial, sans-serif;
      -webkit-font-smoothing: antialiased;
      -moz-osx-font-smoothing: grayscale;

      color: #9F9FA4;
      height: 100%;

      text-shadow: 0 0 2px #000000, 0 0 2px #000000, 0 0 2px #000000, 0 0 2px #000000;
    }

    html {
      background-color: black;
      height: 100%;
    }

    body {
      height: 100%;
      user-select: none;
      overflow: hidden;
    }

    * {
      margin: 0;
      padding: 0;
    }

    ::-webkit-scrollbar {
        width: 12px;
    }

    ::-webkit-scrollbar-thumb:vertical {
        background: radial-gradient(circle, #4c4c82, #181846);
        box-shadow: inset 1px 0 #0E0B32, inset 0 0 #0E0B32, inset 2px 1px 2px hsla(0, 0%, 100%, .15), inset -2px -1px 2px rgba(0, 0, 0, .15);
    }

    ::-webkit-scrollbar-thumb:hover {
        background: radial-gradient(circle, #5c5c92, #282856);
    }

    ::-webkit-scrollbar-track:vertical {
        background: black;
        border-left: 1px solid #0a133d;
    }

    ::-webkit-scrollbar-track:horizontal {
        background: #808094;
        border-top: 1px solid #0a133d;
    }

    ::-webkit-scrollbar-corner {
        background: #808094;
    }

</style>
