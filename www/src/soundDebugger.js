import { opcode_name } from "wasm-gameboy-emulator/wasm_gameboy_emulator";
import { compareUint8Array, interestingRanges, toHex } from "./utils.js";
import React, { useState } from "react";

const SoundDebugger = props => {
  const { fullMemory } = props;

  const numToEightBitsBinary = num => {
    const str = num.toString(2);
    if (str.length < 8) {
      return "0".repeat(8 - str.length) + str;
    }

    return str;
  };

  const numToHex = num => {
    return "0x" + num.toString(16);
  };

  const hexToNum = hex => {
    return parseInt(hex, 16);
  };

  return (
    <div>
      <table className="channel">
        <tbody>
          <tr>
            <th>Address</th>
            <th>Hex</th>
            <th>Binary</th>
          </tr>
          <tr>
            <th>ff10</th>
            <th>{numToHex(fullMemory[hexToNum("ff10")])}</th>
            <th className="binary-val">
              <i>-PPPNSSS</i>
              <br></br>
              {numToEightBitsBinary(fullMemory[hexToNum("ff10")])}
            </th>
          </tr>
          <tr>
            <th>ff11</th>
            <th>{numToHex(fullMemory[hexToNum("ff11")])}</th>
            <th className="binary-val">
              <i>DDLLLLLL</i>
              <br></br>
              {numToEightBitsBinary(fullMemory[hexToNum("ff11")])}
            </th>
          </tr>
          <tr>
            <th>ff12</th>
            <th>{numToHex(fullMemory[hexToNum("ff12")])}</th>
            <th className="binary-val">
              <i>VVVVAPPP</i>
              <br></br>
              {numToEightBitsBinary(fullMemory[hexToNum("ff12")])}
            </th>
          </tr>
          <tr>
            <th>ff13</th>
            <th>{numToHex(fullMemory[hexToNum("ff13")])}</th>
            <th className="binary-val">
              <i>FFFFFFFF</i>
              <br></br>
              {numToEightBitsBinary(fullMemory[hexToNum("ff13")])}
            </th>
          </tr>
          <tr>
            <th>ff14</th>
            <th>{numToHex(fullMemory[hexToNum("ff14")])}</th>
            <th className="binary-val">
              <i>TL---FFF</i>
              <br></br>
              {numToEightBitsBinary(fullMemory[hexToNum("ff14")])}
            </th>
          </tr>
        </tbody>
      </table>
    </div>
  );
};

export { SoundDebugger };
