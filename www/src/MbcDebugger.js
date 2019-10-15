import {opcode_name} from 'wasm-gameboy-emulator/wasm_gameboy_emulator';
import {compareUint8Array, interestingRanges, toHex} from './utils.js';
import React, {useState} from 'react';

const MbcDebugger = props => {
  const {mbc, romBank, ramBank, isRamEnabled, isRomEnabled} = props;

  return (
    <div className="break-point-wrapper">
      <h3>MBC Values</h3>
      <table>
        <tbody>
          <tr>
            <td>MBC type:</td>
            <td>{mbc}</td>
          </tr>
          <tr>
            <td>Rom bank:</td>
            <td>{romBank}</td>
          </tr>
          <tr>
            <td>Ram bank:</td>
            <td>{ramBank}</td>
          </tr>
          <tr>
            <td>ram?</td>
            <td>{String(isRamEnabled)}</td>
          </tr>
          <tr>
            <td>rom?</td>
            <td>{String(isRomEnabled)}</td>
          </tr>
        </tbody>
      </table>
    </div>
  );
};
export {MbcDebugger};
