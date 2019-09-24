[@bs.send] external repeat: (string, int) => string = "repeat";
[@bs.send] external _stringOfInt: (int, int) => string = "toString";
[@bs.send] external padStart: (string, int, string) => string = "toString";

let stringOfInt = (~base=10, number: int, ()) => _stringOfInt(number, base);

let toHex = (num, len) => {
  let str = stringOfInt(num, ());
  repeat("0", max(len - String.length(str), 0)) ++ str;
};

let toPaddedHex = (num, len) =>
  num->stringOfInt(~base=16, _, ())->padStart(len, "0");

let toPaddedBinary = (num, len) =>
  num->stringOfInt(~base=2, _, ())->padStart(len, "0");
