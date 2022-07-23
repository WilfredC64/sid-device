<!--
  Copyright (C) 2022 Wilfred Bos
  Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.
-->

<template>
    <input :id="id" type="checkbox" v-model="checked" @change="emitValue">
    <label :for="id">{{ label }}</label>
</template>

<script>

export default {
    name: 'check-box',
    props: {
        id: {
            type: String,
            default: ""
        },
        checked: {
            type: Boolean,
            default: false
        },
        label: {
            type: String,
            default: ""
        },
    },
    emits: ["change"],
    setup(props, ctx) {
        const emitValue = (event) => {
            ctx.emit('change', event);
        };

        return {
            emitValue
        }
    }
};

</script>

<style scoped>

label {
    padding-left: 6px;
    font-size: 17px;
}

label:hover {
    cursor: pointer;
}

input[type="checkbox"]:hover {
    cursor: pointer;
}

input[type="checkbox"]:after {
    border: 1px solid #26262A;
    background-color: rgba(235, 235, 255, .4);
}

input[type="checkbox"]:hover:after, input[type="checkbox"]:focus:after {
    background-color: rgba(235, 235, 255, .4);
}

input[type="checkbox"]:hover + label {
    color: #d4d0e0;
}

input[type="checkbox"]:focus + label {
    color: #d4d0e0;
}

input[type="checkbox"] {
    appearance: none;
    -webkit-appearance: none;
    margin: 0;
    display: flex;
}

input[type="checkbox"] + label {
    margin-top: 1px;
    margin-left: 3px;
    height: 19px;
}

input[type="checkbox"]:after {
    content: "\00a0";
    display: inline-flex;
    justify-content: center;
    align-items: center;
    height: 15px;
    width: 15px;
    color: white;
    border-radius: 2px;
    outline: none;
    font-size: 11px;
    margin: 0;
    padding: 0;
}

input[type="checkbox"]:checked:hover:after, input[type="checkbox"]:checked:focus:after {
    background-color: rgba(235, 235, 255, .20);
    border: 1px solid #232240;
}

input[type="checkbox"]:checked:after {
    color: #e8eaf0;
    content: "\2714";
    text-align: center;
    background-color: rgba(235, 235, 255, .20);
    border: 1px solid #232240;
    text-shadow: 0 0 3px #000000, 0 0 3px #000000, 0 0 3px #000000, 0 0 3px #000000;
}

</style>
