# Level crossing controller

This document describes what the installation shows to the road and how the
barrier is driven. It is written as paragraphs rather than as tables, and it
compares values in words, because that is how this discipline writes.

## Road signal

While the installation is powered, the road signal follows the approach
detector. When IN_TrainApproach is APPROACHING the road signal is FLASHING.
When IN_TrainApproach is OCCUPIED the road signal is STEADY. When
IN_TrainApproach is CLEAR the road signal is DARK.

## Bell

The bell rings whenever the road signal is not DARK, so that a road user who
is not looking at the signal is still warned. When IN_TrainApproach is CLEAR
the bell is SILENT.

## Barrier

The barrier is driven from the approach detector and from the obstacle
detector together, because the barrier must never come down on a vehicle.

When IN_TrainApproach is APPROACHING and IN_ObstacleDetector is NONE the
barrier command is LOWER. When IN_BarrierPosition is DOWN the barrier command
is STOP, since there is nothing further to do.

When IN_ObstacleDetector is BLOCKED the barrier command is RAISE and the road
signal stays FLASHING until the obstruction clears.

When IN_TrainApproach is CLEAR the barrier command is RAISE.

## Local override

A maintainer may take local control. While IN_LocalOverride is ON the barrier
command is STOP and the road signal is DARK.

## Recovery

After the obstruction clears the barrier waits 8 seconds before lowering
again, so that a vehicle that has just moved off the crossing is clear of it.
