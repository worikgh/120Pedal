For each pedal that the hardware has create a file in this directory `pedal_<N>`. E.g: the SINCO pedal in the [examples](../examples/sinco_pedal) has four switches, 1..4. So create:

```
pedal_1
pedal_2
pedal_3
pedal_4
```

This is important so the system will know how to allocate pedals and display them in the UI

Pedal definition files look like:
```
system:capture_1 effect_8:in
effect_9:output qzn3t_mixer:input_1
```

Each line can be used as an argument to `jack_connect`.  That will activate the pedal simulator.
