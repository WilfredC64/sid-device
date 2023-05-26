<!--
  Copyright (C) 2022 - 2023 Wilfred Bos
  Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.
-->

<template>
    <div id="about" ref="about">
        <div id="content">
            <img alt="Sid-Device logo" class="logo" draggable="false" ondragstart="return false;" src="../assets/sid_device_256x256.png">
            <div class="title-wrapper">
                <h1 class="title">SID Device v1.0.1</h1>
            </div>
            <div class="credits">
                <p>
                    Copyright &#xa9; 2021 - 2023 by Wilfred Bos
                </p>
                <p>
                    Network SID Interface &ndash; Copyright &#xa9; 2007 - 2023
                    <br/>
                    by Wilfred Bos, Ken H&auml;ndel and Antti S. Lankila
                </p>
                <p>
                    reSID v1.0 &ndash; Copyright &#xa9; 1998 - 2023 by Dag Lem
                </p>
            </div>
        </div>
    </div>
</template>

<script>

import { listen } from '@tauri-apps/api/event';
import { ref } from 'vue'

export default {
    name: 'AboutDialog',
    setup() {
        const about = ref();

        const activateListeners = async () => {
            await listen('show', async () => {
                if (about.value) {
                    about.value.style.display = 'block';
                }
            });

            await listen('hide', async () => {
                if (about.value) {
                    about.value.style.display = 'none';
                }
            });
        }

        activateListeners();

        return {
            about
        }
    }
}

</script>

<style scoped>

#about {
    background-color: #010105;
    background-image: url("../assets/about_background.png");
    background-repeat: no-repeat;
    background-size: 100% 500px;
    background-position: bottom center;
    background-attachment: fixed;
    display: none;

    text-align: center;
    overflow-y: auto;
}

#content {
    margin: 10px 0;
}

p {
    padding: 8px 0;
}

.logo {
    margin: 16px 0 0 0;
}

.credits {
    padding-bottom: 16px;
    position: absolute;
    width: 100%;
    bottom: 0;
}

.title-wrapper {
    position: relative;
    z-index: 40;
}

.title {
    font-size: 36px;
    padding: 4px 0;
    line-height: 50px;
    color: #3838a0;
    background: linear-gradient(#444450 5%, #dfdff4 20%, #dfdff4 80%, #444450 95%);
    text-shadow: 0 0 2px #000000, 0 0 2px #000000, 0 0 2px #000000, 0 0 2px #000000, 0 0 2px #000000, 0 0 2px #000000;
    background-clip: content-box;
    position: absolute;
    width: 100%;
    animation: upDown 1.5s alternate infinite ease-in-out;
}

@keyframes upDown {
    to { transform: translatey(32px); }
}

</style>
