import {opcode_name} from 'wasm-gameboy-emulator/wasm_gameboy_emulator';
import {compareUint8Array, interestingRanges, toHex} from './utils.js';
import React, {useState} from 'react';

const SoundDebugger = props => {
  const {
    fullMemory,
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
    is_use_length,
  } = props;

  const numToEightBitsBinary = num => {
    const str = num.toString(2);
    if (str.length < 8) {
      return '0'.repeat(8 - str.length) + str;
    }

    return str;
  };

  const numToHex = num => {
    return '0x' + num.toString(16);
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
            <th>{numToHex(fullMemory[hexToNum('ff10')])}</th>
            <th className="binary-val">
              <i>-PPPNSSS</i>
              <br />
              {numToEightBitsBinary(fullMemory[hexToNum('ff10')])}
            </th>
          </tr>
          <tr>
            <th>ff11</th>
            <th>{numToHex(fullMemory[hexToNum('ff11')])}</th>
            <th className="binary-val">
              <i>DDLLLLLL</i>
              <br />
              {numToEightBitsBinary(fullMemory[hexToNum('ff11')])}
            </th>
          </tr>
          <tr>
            <th>ff12</th>
            <th>{numToHex(fullMemory[hexToNum('ff12')])}</th>
            <th className="binary-val">
              <i>VVVVAPPP</i>
              <br />
              {numToEightBitsBinary(fullMemory[hexToNum('ff12')])}
            </th>
          </tr>
          <tr>
            <th>ff13</th>
            <th>{numToHex(fullMemory[hexToNum('ff13')])}</th>
            <th className="binary-val">
              <i>FFFFFFFF</i>
              <br />
              {numToEightBitsBinary(fullMemory[hexToNum('ff13')])}
            </th>
          </tr>
          <tr>
            <th>ff14</th>
            <th>{numToHex(fullMemory[hexToNum('ff14')])}</th>
            <th className="binary-val">
              <i>TL---FFF</i>
              <br />
              {numToEightBitsBinary(fullMemory[hexToNum('ff14')])}
            </th>
          </tr>
        </tbody>
      </table>
      <table className="channle-summary">
        <tbody>
          <tr>
            <th>sweep_time:</th>
            <th>{sweep_time}</th>
          </tr>
          <tr>
            <th>is_sweep_increase</th>
            <th>{is_sweep_increase ? 'true' : 'false'}</th>
          </tr>
          <tr>
            <th>sweep_shift_num </th>
            <th>{sweep_shift_num}</th>
          </tr>
          <tr>
            <th>wave_duty_pct</th>
            <th>{wave_duty_pct}</th>
          </tr>
          <tr>
            <th>sound_length_sec</th>
            <th>{sound_length_sec}</th>
          </tr>
          <tr>
            <th>volume</th>
            <th>{volume}</th>
          </tr>
          <tr>
            <th>is_envelop_increase</th>
            <th>{is_envelop_increase ? 'true' : 'false'}</th>
          </tr>
          <tr>
            <th>envelop_shift_num</th>
            <th>{envelop_shift_num}</th>
          </tr>
          <tr>
            <th>frequency(raw)</th>
            <th>{fr}</th>
          </tr>
          <tr>
            <th>frequency</th>
            <th>{frequency}</th>
          </tr>
          <tr>
            <th>is_restart</th>
            <th>{is_restart ? 'true' : 'false'}</th>
          </tr>
          <tr>
            <th>is_use_length</th>
            <th>{is_use_length ? 'true' : 'false'}</th>
          </tr>
        </tbody>
      </table>
    </div>
  );
};

export {SoundDebugger};
