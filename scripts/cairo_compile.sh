#!/bin/bash

echo "Compiling Cairo Program"
cairo-compile --cairo_path=cairo/packages/garaga_zero/src "cairo/src/test_hash_to_field.cairo" --output "cairo/build/main.json" --proof_mode


if [ $? -eq 0 ]; then
    echo "Compilation Successful!"
    
fi
