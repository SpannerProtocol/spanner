
// Reads in the type definitions from all pallets in the runtime and the runtime's own types
// Naively aggregates types and writes them to disk.

const fs = require('fs');
const {exec} = require("child_process");

// A list of all the installed modules with custom types
// Does not include system pallets because Apps already supports them.
// Redundant with construct_runtime!
const folders = [
    "primitives",
    "pallets/bullet-train",
    "pallets/rewards",
    "pallets/support",
    "pallets/dex"
]

let finalTypes = {};
let customTypes;
for (let dirname of folders) {
    let path = `../${dirname}/types.json`;
    customTypes = JSON.parse(fs.readFileSync(path, 'utf8'));
    finalTypes = {...finalTypes, ...customTypes};
}

// Write output to disk
fs.writeFileSync("../types.json", JSON.stringify(finalTypes, null, 2), 'utf8');

// Convert to types mapping and output to disk
exec("python3 convert.py ../types.json > ../types_mapping.json");
