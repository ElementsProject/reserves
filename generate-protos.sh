#!/bin/sh

protoc --rust_out ./src/protos protos/reserves.proto
