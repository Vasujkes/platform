#!/usr/bin/env bash

TARGET=wasm32-unknown-unknown

PROFILE_ARG=""
PROFILE="debug"

if [ -n "$CARGO_BUILD_PROFILE" ]; then
    if [ "$CARGO_BUILD_PROFILE" == "release" ]; then
      PROFILE_ARG="--release"
      PROFILE="release"
    elif [ "$CARGO_BUILD_PROFILE" != "debug" ]; then
      PROFILE_ARG="--profile $CARGO_BUILD_PROFILE"
      PROFILE="$CARGO_BUILD_PROFILE"
    fi
fi

OUTPUT_DIR="$PWD/wasm"
OUTPUT_FILE="$OUTPUT_DIR/wasm_dpp_bg.wasm"
OUTPUT_FILE_JS="$OUTPUT_DIR/wasm_dpp_bg.js"
BUILD_COMMAND="cargo build --target=$TARGET $PROFILE_ARG"
BINDGEN_COMMAND="wasm-bindgen --out-dir=$OUTPUT_DIR --target=web --omit-default-module-path ../../target/$TARGET/$PROFILE/wasm_dpp.wasm"

DIST_TYPINGS="$PWD/dist/wasm/wasm_dpp.d.ts"
WASM_TYPINGS_PATH="$OUTPUT_DIR/wasm_dpp.d.ts"


if ! [[ -d $OUTPUT_DIR ]];  then
  mkdir -p "$OUTPUT_DIR"
fi

if ! [ -x "$(command -v wasm-bindgen)" ]; then
    echo 'Wasm-bindgen CLI is not installed. Installing';
    cargo install -f wasm-bindgen-cli
fi


# On a mac, bundled clang won't work - you need to install LLVM manually through brew,
# and then set the correct env for the build to work
if [[ "$OSTYPE" == "darwin"* ]]; then
    AR_PATH=$(which llvm-ar)
    CLANG_PATH=$(which clang)
    AR=$AR_PATH CC=$CLANG_PATH $BUILD_COMMAND
    AR=$AR_PATH CC=$CLANG_PATH $BINDGEN_COMMAND
else
    $BUILD_COMMAND
    $BINDGEN_COMMAND
fi

# EMCC_CFLAGS="-s ERROR_ON_UNDEFINED_SYMBOLS=0 --no-entry" cargo build --target=wasm32-unknown-emscripten --release
# EMCC_CFLAGS="-s ERROR_ON_UNDEFINED_SYMBOLS=0 --no-entry" wasm-bindgen --out-dir=wasm --target=web --omit-default-module-path ../../target/wasm32-unknown-emscripten/release/wasm_dpp.wasm

# TODO: Must be somehow preinstalled?
#if [ "$PROFILE" == "release" ]; then
#  echo "Optimizing wasm using Binaryen"
#  wasm-opt -Os "$OUTPUT_FILE" -o "$OUTPUT_FILE"
#fi

# Converting wasm into base64 so it can be bundled
echo "Converting wasm binary into base64 module for bundling with Webpack"
WASM_BUILD_BASE_64=$(base64 -i "$OUTPUT_FILE")
echo 'module.exports = "'${WASM_BUILD_BASE_64}'"' > "$OUTPUT_FILE_JS"

mkdir "$PWD/dist/"

echo "Copying wasm typings"
mkdir "$PWD/dist/wasm"
cp "$WASM_TYPINGS_PATH" "$DIST_TYPINGS"
echo "Typings copied"
