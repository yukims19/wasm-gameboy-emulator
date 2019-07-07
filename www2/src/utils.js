const toHex = (num, len) => {
  var str = num.toString(16);
  return "0".repeat(Math.max(len - str.length, 0)) + str;
};

const compareUint8Array = (a, b) => {
  if (a.length !== b.length) {
    throw new Exception("Arrays are different length, cannot compare");
  }

  const diffList = [];

  const aLength = a.length;

  for (var idx = 0; idx < aLength; idx++) {
    if (a[idx] !== b[idx]) {
      diffList.push({ idx: toHex(idx, 4), a: a[idx], b: b[idx] });
    }
  }

  return diffList;
};

const interestingRanges = {
  interruptEnableFlags: {
    range: [0xffff, 0xffff],
    desc: "Interrupt Enable Flags"
  },
  bootRom: { range: [0x00, 0x00ff], desc: "Bootrom" },
  highRam: { range: [0xff80, 0xfffe], desc: "High RAM" },
  ioRegisters: { range: [0xff00, 0xff7f], desc: "Hardware IO Registers" },
  unusuableMemory: { range: [0xfea0, 0xfeff], desc: "Unusable Memory" },
  oam: { range: [0xfe00, 0xfe9f], desc: "OAM (Object Attribute Memory)" },
  echo: { range: [0xe000, 0xfdff], desc: "Echo RAM - Do not use" },
  ramBanks1to7: {
    range: [0xd000, 0xdfff],
    desc: "Internal RAM Banks 1-7 switchable"
  },
  ramBank0: { range: [0xc000, 0xcfff], desc: "Internal RAM Bank 0 Fixed" },
  cartridgeRam: {
    range: [0xa000, 0xbfff],
    desc: "Cartridge RAM (if available)"
  },
  backgroundMap2: { range: [0x9c00, 0x9fff], desc: "Background Map Data 2" },
  backgroundMap1: { range: [0x9800, 0x9bff], desc: "Background Map Data 1" },
  characterRam: { range: [0x8000, 0x97ff], desc: "Character RAM" },
  cartridgeRom: { range: [0x4000, 0x7fff], desc: "Cartridge ROM switchable" },
  cartridgeRomBank0: {
    range: [0x0150, 0x3fff],
    desc: "Cartridge ROM Bank 0 fixed"
  },
  cartridgeHeader: { range: [0x0100, 0x014f], desc: "Cartridge Header" },
  interruptVectors: { range: [0x0000, 0x00ff], desc: "Interrupt vectors" }
};

export { compareUint8Array, interestingRanges, toHex };
