# Mono Effects

This directory contains executable file that start effects that take one Jack input and one Jack output

The name of the executable file, herein, is the root name of the Jack pipes

The variables for the models are defined as shell variables.  For example in the `chorus` model there are two parameters: `SPEED` and `DEPTH`.

```sh
SPEED=2.5
DEPTH=3
```

The parameters can then be changed programmatically using `sed`

`sed -i 's/SPEED=.*$/SPEED=1.5/' mono/chorus`
