#!/bin/bash

export One20PedalHome=/home/patch/120Pedal
export PEDAL_DIR=${One20PedalHome}/PEDALS
export PEDAL=${One20PedalHome}/midi_driver/target/release/midi_driver
export MODEP_PEDALS=/var/modep/pedalboards
export LV2_PATH=${MODEP_PEDALS}:/var/modep/lv2
export MODHOST_PORT=8302

