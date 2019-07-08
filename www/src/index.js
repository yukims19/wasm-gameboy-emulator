import {
  Canvas,
  Pixel,
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

var domContainer = document.querySelector("#memory-viewer");

// TODO: Move this into the Rust side
let tick = -1;
let isPlaying = false;
const opLogMaxLength = 16;
const opLog = [];

var render = function render(gameboy) {
  var memoryPtr = gameboyInst.memory();
  var memoryBytes = new Uint8Array(memory.buffer, memoryPtr, 65535);

  tick = tick + 1;

  const next = () => {
    gameboy.execute_opcodes(1000);
    updateCharMapCanvas(gameboy);
    renderBackgroundMap1AsImageData(gameboy, memoryBytes);

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

  // A debugging handle to play with in the console
  window.fullMemory = () => memoryBytes;

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
      }
    }),
    domContainer
  );

  if (isPlaying) {
    next();
  }
};

requestAnimationFrame(() => render(gameboyInst));
