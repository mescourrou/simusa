#!/bin/env python3

import simusa
import json
import numpy as np
import math

class Navigator(simusa.Navigator):
    def __init__(self, config: dict, initial_time: float):
        if "radius" in config:
            self.radius = config["radius"]
        else:
            self.radius = 10
        if "velocity" in config:
            self.velocity = config["velocity"]
        else:
            self.velocity = 10
    
    def post_init(self, node: simusa.Node) -> None:
        pass

    def record(self) -> str:
        print("This is record from python!")
        return json.dumps({})

    def compute_output(self, node: simusa.Node, world_state: simusa.WorldState) -> simusa.ControllerInput:
        output = simusa.ControllerInput()
        v = world_state.ego.velocity.x
        x = world_state.ego.pose.x
        y = world_state.ego.pose.y
        theta = world_state.ego.pose.theta
        
        # Very bad error computation, but that's an example
        pose = np.array([x, y, theta])
        r = np.linalg.norm(pose[:2])
        output.error_lateral = self.radius - r
        output.error_longitudinal = 0.
        targetted_theta = math.atan2(y, x) + np.pi/2
        output.error_angular = ((targetted_theta - theta) + np.pi) % (2*np.pi)
        output.error_linear_velocity = self.velocity - v
        return output

    def pre_loop_hook(self, node: simusa.Node, time: float):
        pass
    
    def next_time_step(self):
        return None

class SimulatorAPI(simusa.PluginAPI):
    def get_navigator(self, config: dict, global_config: dict, initial_time: float):
        config = json.loads(config)
        print(f"Config received by python: {type(config)} {config}")
        return Navigator(config, initial_time)

def main():

    simulator_api = SimulatorAPI()

    simulator = simusa.Simulator.from_config(
        "config/config_navigator.yaml", simulator_api
    )
    simulator.run()


if __name__ == "__main__":
    main()
