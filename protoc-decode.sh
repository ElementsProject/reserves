#!/bin/sh

protoc --decode ProofOfReserves protos/reserves.proto < reserves.proof
