import {
  Canvas,
  Pixel,
  FmOsc,
  init as initEmulation,
  opcode_name
} from "wasm-gameboy-emulator/wasm_gameboy_emulator";
import { memory } from "wasm-gameboy-emulator/wasm_gameboy_emulator_bg";
import React, { useState } from "react";
import {
  areTypedArraysEqual,
  compareUint8Array,
  interestingRanges,
  toHex
} from "./utils.js";
import { Debugger } from "./debugControls.js";
import { SoundDebugger } from "./soundDebugger.js";
import { square1, playSquare, playSquare1, playSquare2 } from "./channels.jsx";

import("wasm-gameboy-emulator/wasm_gameboy_emulator");

let fmOsc = new FmOsc();

const setSound = square1 => {
  var volume = new Float32Array(1); //square1.volume() / 10;
  volume[0] = 0.8;
  let is_envelop_increase = square1.is_envelop_increase();
  let envelop_shift_num = square1.envelop_shift_num();

  console.log(volume[0], is_envelop_increase, envelop_shift_num);

  const play_button = document.getElementById("play");
  play_button.addEventListener("click", event => {
    if (fmOsc === null) {
      console.log("playsound");
      fmOsc = new FmOsc();
      fmOsc.set_primary_frequency(1551);
      fmOsc.set_gain(volume[0]);
      const fm_amount = document.getElementById("fm_amount");
      fm_amount.addEventListener("input", event => {
        if (fmOsc) {
          const shiftNum = 3;
          console.log(
            `GainShift volume=${volume}, shiftNum=${shiftNum}, is_envelop_increase=${is_envelop_increase}`
          );
          fmOsc.set_gain_shift(0.8, shiftNum, is_envelop_increase);
          const fmVolume = fmOsc.volume();
          const fmFrequency = fmOsc.frequency();
          console.log("fm:", fmVolume);
          console.log("fm2:", fmFrequency);
        }
      });
    } else {
      fmOsc.free();
      fmOsc = null;
    }
  });
};

// .then(rust_module => {
//   let fm = null;

//   const play_button = document.getElementById("play");
//   play_button.addEventListener("click", event => {
//     if (fm === null) {
//       fm = new rust_module.FmOsc();
//       fm.set_primary_frequency(1551);
//       // fm.set_fm_frequency(0);
//       // fm.set_fm_amount(0);
//       fm.set_gain(0.8);
//     } else {
//       fm.free();
//       fm = null;
//     }
//   });

// const primary_slider = document.getElementById("primary_input");
// primary_slider.addEventListener("input", event => {
//   if (fm) {
//     fm.set_note(parseInt(event.target.value));
//   }
// });

// const fm_freq = document.getElementById("fm_freq");
// fm_freq.addEventListener("input", event => {
//   if (fm) {
//     fm.set_fm_frequency(parseFloat(event.target.value));
//   }
// });

// const fm_amount = document.getElementById("fm_amount");
// fm_amount.addEventListener("input", event => {
//   if (fm) {
//     fm.set_fm_amount(parseFloat(event.target.value));
//   }
// });
// })

var ReactDOM = require("react-dom");

const config = {
  PIXEL_ZOOM: 1
};

initEmulation();
const gameboyInst = Canvas.new();

const makeCanvas = (canvasSelector, options) => {
  console.log("Making canvas from ", canvasSelector);
  const el = document.querySelector(canvasSelector);
  const ctx = el.getContext("2d");
  const zoom = options.zoom || 1;

  el.width = options.width;
  el.height = options.height;
  el.style.width = el.width * zoom + "px";
  el.style.height = el.height * zoom + "px";

  ctx["imageSmoothingEnabled"] = false; /* standard */
  ctx["oImageSmoothingEnabled"] = false; /* Opera */
  ctx["webkitImageSmoothingEnabled"] = false; /* Safari */
  ctx["msImageSmoothingEnabled"] = false; /* IE */

  return [el, ctx];
};

var background_width = gameboyInst.background_width();
var background_height = gameboyInst.background_height();
var screen_width = gameboyInst.screen_width();
var screen_height = gameboyInst.screen_height();

const [backgroundCanvasEl, backgroundCanvas] = makeCanvas(
  "#gameboy-background-canvas",
  {
    width: config.PIXEL_ZOOM * background_width,
    height: config.PIXEL_ZOOM * background_height
  }
);
const [screenCanvasEl, screenCanvas] = makeCanvas("#gameboy-screen-canvas", {
  width: config.PIXEL_ZOOM * screen_width,
  height: config.PIXEL_ZOOM * screen_height
});
const [charMapCanvasEl, charMapCanvas] = makeCanvas("#char-map-actual-canvas", {
  width: 8,
  height: 1024
});
const [charMapDebugCanvasEl, charMapDebugCanvas] = makeCanvas(
  "#char-map-debug-canvas",
  { width: 8 * 12, height: 8 * 8, zoom: 4 }
);

const clearContext = context => {
  context.clearRect(0, 0, context.canvas.width, context.canvas.height);
};

const getTileImageData = (canvas, tileIdx) => {
  const y0 = tileIdx * 8;
  const imageData = canvas.getImageData(0, y0, 8, y0 + 8);
  return imageData;
};

// Quick hack to memoize updateCharMapCanvas
// Should move this stuff to the Rust side later
let updateCharMapCanvas_lastData = null;

const updateCharMapCanvas = gameboy => {
  const rustImageData = gameboy.char_map_to_image_data();
  const imageSource = new Uint8ClampedArray(rustImageData);

  const hasChanged =
    !!updateCharMapCanvas_lastData &&
    !areTypedArraysEqual(updateCharMapCanvas_lastData, rustImageData);

  if (hasChanged) {
    const imageData = new ImageData(imageSource, 8);
    charMapCanvas.putImageData(imageData, 0, 0);
    const tilesPerRow = 12;
    clearContext(charMapDebugCanvas);
    for (var tileIdx = 0; tileIdx < 96; tileIdx++) {
      const tile = getTileImageData(charMapCanvas, tileIdx);
      const x = tileIdx % tilesPerRow;
      const y = Math.floor(tileIdx / tilesPerRow);
      charMapDebugCanvas.putImageData(tile, x * 8, y * 8);
    }
  }

  updateCharMapCanvas_lastData = rustImageData;
};

const drawScreen = () => {
  clearContext(screenCanvas);
  const isLcdEnable = gameboyInst.is_lcd_display_enable();
  if (!isLcdEnable) {
    return;
  }

  var x = gameboyInst.get_scroll_x();
  var y = gameboyInst.get_scroll_y();

  const imageData = backgroundCanvas.getImageData(
    x,
    y,
    config.PIXEL_ZOOM * screen_width,
    config.PIXEL_ZOOM * screen_height
  );

  screenCanvas.putImageData(imageData, 0, 0);
};

// Quick hack to memoize renderBackgroundMap1AsImageData
// Should move this stuff to the Rust side later
let lastBackgroundMap1 = null;

const renderBackgroundMap1AsImageData = (gameboy, fullMemory) => {
  const backgroundMap1 = gameboy.background_map_1();

  const hasChanged =
    !!lastBackgroundMap1 &&
    !areTypedArraysEqual(lastBackgroundMap1, backgroundMap1);

  if (!lastBackgroundMap1) {
    lastBackgroundMap1 = Uint8Array.from(backgroundMap1);
  }

  if (hasChanged) {
    lastBackgroundMap1 = Uint8Array.from(backgroundMap1);

    const tiles = [];

    for (var idx = 0; idx < 32 * 32; idx++) {
      tiles.push(getTileImageData(charMapCanvas, idx));
    }

    clearContext(backgroundCanvas);

    let x = 0;
    let y = 0;
    backgroundMap1.forEach(function(ele, idx) {
      const tile = tiles[ele];
      backgroundCanvas.putImageData(tile, x, y);

      x = x + 8;
      if (x >= 32 * 8) {
        x = 0;
        y = y + 8;
      }
    });
  }

  drawScreen();
};

const playSound = gameboy => {
  // //TODO:Need to fix sound on/off timing
  // if (
  //   gameboy.is_sound_all_on() // && gameboy.square1().fr() !== 0
  // ) {
  //   const audioCtx = new AudioContext();
  //   playSquare(audioCtx, gameboy.square1());
  // } else if (gameboy.is_sound_1_on()) {
  //   console.log("sound 1 on");
  // } else {
  //   console.log(".");
  // }
};

var domContainer = document.querySelector("#memory-viewer");
var soundContainer = document.getElementById("sound-container");

// TODO: Move this into the Rust side
let tick = -1;
let isPlaying = false;
const opLogMaxLength = 16;
const opLog = [];

var render = function render(gameboy) {
  var memoryPtr = gameboyInst.memory();
  var memoryBytes = new Uint8Array(memory.buffer, memoryPtr, 65535);

  tick = tick + 1;

  setSound(gameboy.square1());

  const next = () => {
    gameboy.execute_opcodes(1000);
    updateCharMapCanvas(gameboy);
    renderBackgroundMap1AsImageData(gameboy, memoryBytes);
    playSound(gameboy);

    requestAnimationFrame(() => render(gameboy, memoryBytes));
  };

  var pc = gameboy.get_pc();
  const opcodeLogEntry = {
    tick: tick,
    pc: pc,
    opcode: memoryBytes[pc],
    memory: new Uint8Array(memoryBytes)
  };

  opLog.push(opcodeLogEntry);
  if (opLog.length > opLogMaxLength) {
    // Take out the oldest item with shift
    opLog.shift();
  }

  var nextPc = gameboy.get_pc();

  const onStep = () => next();
  const onTogglePlay = () => {
    isPlaying = !isPlaying;
    next();
  };

  const registers = {
    a: gameboy.get_a(),
    b: gameboy.get_b(),
    c: gameboy.get_c(),
    d: gameboy.get_d(),
    e: gameboy.get_e(),
    h: gameboy.get_h(),
    l: gameboy.get_l(),
    sp: gameboy.get_sp(),
    pc: gameboy.get_pc(),
    flags: {
      z: gameboy.get_flag_z(),
      n: gameboy.get_flag_n(),
      h: gameboy.get_flag_h(),
      c: gameboy.get_flag_c()
    }
  };

  const cycleTotal = gameboy.total_cycle();
  const timer = gameboy.timer();
  const cpuClock = gameboy.cpu_clock();

  // A debugging handle to play with in the console
  window.fullMemory = () => memoryBytes;

  const square1 = gameboy.square1();
  let sweep_time = square1.sweep_time();
  let is_sweep_increase = square1.is_sweep_increase();
  let sweep_shift_num = square1.sweep_shift_num();
  let wave_duty_pct = square1.wave_duty_pct();
  let sound_length_sec = square1.sound_length_sec();
  let volume = square1.volume() / 10;
  let is_envelop_increase = square1.is_envelop_increase();
  let envelop_shift_num = square1.envelop_shift_num();
  let fr = square1.fr();
  let frequency = square1.frequency();
  let is_restart = square1.is_restart();
  let is_use_length = square1.is_use_length();

  ReactDOM.render(
    React.createElement(SoundDebugger, {
      fullMemory: memoryBytes,
      sweep_time,
      is_sweep_increase,
      sweep_shift_num,
      wave_duty_pct,
      sound_length_sec,
      volume,
      is_envelop_increase,
      envelop_shift_num,
      fr,
      frequency,
      is_restart,
      is_use_length
    }),
    soundContainer
  );

  ReactDOM.render(
    React.createElement(Debugger, {
      tick: tick,
      isPlaying: isPlaying,
      onStep: onStep,
      onTogglePlay: onTogglePlay,
      fullMemory: memoryBytes,
      gameboy: gameboy,
      from: 0,
      to: 240,
      pc: pc,
      nextPc: nextPc,
      opLog: opLog,
      registers: registers,
      cycleTotal: cycleTotal,
      timer: timer,
      cpuClock,
      onDraw: () => {
        updateCharMapCanvas(gameboy);
      },
      onClear: () => {
        clearContext(screenCanvas);
        clearContext(backgroundCanvas);
      },
      onDrawBackground: () => {
        // updateCharMapCanvas(gameboy);
        renderBackgroundMap1AsImageData(gameboy, memoryBytes);
      },
      onPlaySound: () => {
        playSound(gameboy);
      }
    }),
    domContainer
  );

  if (isPlaying) {
    next();
  }
};

requestAnimationFrame(() => render(gameboyInst));
