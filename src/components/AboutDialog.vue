<!--
  Copyright (C) 2022 - 2026 Wilfred Bos
  Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.
-->

<template>
    <div id="about">
        <TitleBar />
        <div id="content">
            <img alt="Sid-Device logo" class="logo" draggable="false" ondragstart="return false;" src="../assets/sid_device_256x256.png">
            <div class="title-wrapper">
                <svg class="title" viewBox="0 0 600 50" preserveAspectRatio="xMidYMid meet" xmlns="http://www.w3.org/2000/svg">
                    <defs>
                        <pattern id="titleColorBars" ref="patternEl" patternUnits="userSpaceOnUse" width="600" :height="patternHeight">
                            <rect v-for="(bar, index) in colorBars" :key="index" :y="bar.y" width="600" :height="bar.height" :fill="bar.color" />
                        </pattern>
                    </defs>
                    <text x="50%" y="38" text-anchor="middle" font-size="36" font-weight="bold"
                        fill="url(#titleColorBars)" stroke="#000000" stroke-width="1" paint-order="stroke">SID Device v1.0.3</text>
                </svg>
            </div>
            <div class="credits">
                <p>
                    Copyright &#xa9; 2021 - 2026 by Wilfred Bos
                </p>
                <p>
                    Network SID Interface &ndash; Copyright &#xa9; 2007 - 2026
                    <br/>
                    by Wilfred Bos, Ken H&auml;ndel and Antti S. Lankila
                </p>
                <p>
                    reSID v1.0 &ndash; Copyright &#xa9; 1999 - 2010 by Dag Lem
                </p>
                <p>
                    Nuked-OPL3 &ndash; Copyright &#xa9; 2013 - 2020 by Nuke.YKT
                </p>
            </div>
        </div>
    </div>
</template>

<script>

import TitleBar from './TitleBar.vue';

const C64_COLORS = {
    blue: '#2E2C9B',
    lightBlue: '#706DEB',
    cyan: '#75CEC8',
    white: '#FFFFFF',
    red: '#813338',
    pink: '#C46C71',
    yellow: '#EDF171',
    darkGrey: '#4A4A4A',
    midGrey: '#7B7B7B',
    lightGrey: '#B2B2B2',
    green: '#56AC4D'
};

const RASTER_BARS = [
    [C64_COLORS.blue, C64_COLORS.lightBlue, C64_COLORS.cyan, C64_COLORS.white, C64_COLORS.cyan, C64_COLORS.lightBlue, C64_COLORS.blue],
    [C64_COLORS.red, C64_COLORS.pink, C64_COLORS.yellow, C64_COLORS.white, C64_COLORS.yellow, C64_COLORS.pink, C64_COLORS.red],
    [C64_COLORS.darkGrey, C64_COLORS.green, C64_COLORS.cyan, C64_COLORS.white, C64_COLORS.cyan, C64_COLORS.green, C64_COLORS.darkGrey],
    [C64_COLORS.darkGrey, C64_COLORS.midGrey, C64_COLORS.lightGrey, C64_COLORS.white, C64_COLORS.lightGrey, C64_COLORS.midGrey, C64_COLORS.darkGrey]
];

const SCROLL_SPEED = 12;

function expandBars(waypoints) {
    const steps = [];
    for (let i = 0; i < waypoints.length - 1; i++) {
        steps.push(waypoints[i], waypoints[i + 1], waypoints[i], waypoints[i + 1]);
    }
    return steps;
}

function buildColorSteps(mountains) {
    const steps = [];
    mountains.forEach((mountain, index) => {
        if (index > 0 && steps[steps.length - 1] !== mountain[0]) {
            const from = steps[steps.length - 1];
            const to = mountain[0];
            steps.push(from, to, from, to);
        }
        steps.push(...expandBars(mountain));
    });
    const first = steps[0];
    const last = steps[steps.length - 1];
    if (last !== first) {
        steps.push(last, first, last, first);
    }
    return steps;
}

function toBars(steps) {
    const bars = [];
    let y = 0;
    for (let i = 0; i < steps.length;) {
        let j = i;
        while (j < steps.length && steps[j] === steps[i]) j++;
        bars.push({ y, height: j - i, color: steps[i] });
        y += j - i;
        i = j;
    }
    return bars;
}

const colorBars = toBars(buildColorSteps(RASTER_BARS));
const patternHeight = colorBars.reduce((total, bar) => total + bar.height, 0);

export default {
    name: 'AboutDialog',
    components: {
        TitleBar
    },
    data() {
        return {
            colorBars,
            patternHeight
        }
    },
    mounted() {
        this.startPatternScroll();
    },
    beforeUnmount() {
        cancelAnimationFrame(this.scrollAnimationFrame);
    },
    methods: {
        startPatternScroll() {
            const startTime = performance.now();
            const tick = (now) => {
                const elapsedSeconds = (now - startTime) / 1000;
                const offset = (elapsedSeconds * SCROLL_SPEED) % patternHeight;
                this.$refs.patternEl?.setAttribute('patternTransform', `translate(0, ${-offset})`);
                this.scrollAnimationFrame = requestAnimationFrame(tick);
            };
            this.scrollAnimationFrame = requestAnimationFrame(tick);
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
    text-shadow:
        0 0 4px #000000, 0 0 4px #000000, 0 0 4px #000000,
        0 0 4px #000000, 0 0 4px #000000, 0 0 4px #000000,
        -1px -1px 0 #000000, 1px -1px 0 #000000,
        -1px 1px 0 #000000, 1px 1px 0 #000000;
}

.title-wrapper {
    position: relative;
    z-index: 40;
}

.title {
    display: block;
    height: 50px;
    padding: 4px 0;
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
