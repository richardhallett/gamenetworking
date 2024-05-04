# GameNetworking

This is a basic example of game networking code using faked net code.
This is to demonstrate things like
- client side prediction - Letting the client carry on and predicting input for local player
- reconcilation - Reconciling what server tells us and where client is.
- extrapolation - Extrapolate the position for other entities and interpolate locally

This was a WIP and most likely needs a little more work, namely things like clock synchronisation and lag compensation.