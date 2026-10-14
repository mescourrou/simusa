# Configuration file documentation
This documentation is auto-generated.

To get more information on the parameter, check the documentation (or follow the links).

List of parameters:

`version`: String
`log`: [LoggerConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/logger/struct.LoggerConfig.html)
	`included_nodes`: String, List
	`excluded_nodes`: String, List
	`log_level`: [LogLevel](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/logger/enum.LogLevel.html), Enum
		- `type`: Off  
		- `type`: Error  
		- `type`: Warn  
		- `type`: Info  
		- `type`: Debug  
		- `type`: Internal => [InternalLog](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/logger/enum.InternalLog.html), List, Enum
			- `type`: All  
			- `type`: NetworkMessages  
			- `type`: NetworkMessagesDetailed  
			- `type`: SetupSteps  
			- `type`: SetupStepsDetailed  
			- `type`: SensorManager  
			- `type`: SensorManagerDetailed  
			- `type`: NodeRunning  
			- `type`: NodeRunningDetailed  
			- `type`: NodeSyncDetailed  
			- `type`: API  
			- `type`: NavigatorDetailed  
			- `type`: Scenario  
			- `type`: Environment  
			- `type`: EnvironmentDetailed  
			- `type`: ExternalNetwork  
	`outputs`: [LoggerTypeConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/logger/enum.LoggerTypeConfig.html), List, Enum
		- `type`: File => String
`results`: [ResultConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/simulator/results/struct.ResultConfig.html), Optional
	`result_path`: String, Optional
	`save_mode`: ResultSaveMode
	`analyser`: [ResultAnalyserConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/result_analyser/mod/enum.ResultAnalyserConfig.html), Optional, Enum
		- `type`: Python => [PythonResultAnalyserConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/result_analyser/python_analyser/struct.PythonResultAnalyserConfig.html)
			`show_figures`: Boolean
			`script_path`: String
			`function_name`: String
			`additionnal_params`: GeneralValue
			`port`: Unsigned Integer
			`python_command`: String
		- `type`: Plugin => GeneralValue
`base_path`: String
`max_time`: Float
`optimization`: [OptimizationConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/simulator/simulator_config/struct.OptimizationConfig.html)
	`threads`: usize
`time_analysis`: [TimeAnalysisConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/time_analysis/time_analysis_config/struct.TimeAnalysisConfig.html), Optional
	`exporter`: [ProfileExporterConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/time_analysis/exporter/enum.ProfileExporterConfig.html), Enum
		- `type`: TraceEventExporter  
	`output_path`: String
	`analysis_unit`: AnalysisUnit
`random_seed`: Float, Optional
`nodes`: [NodeConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/node/node_factory/struct.NodeConfig.html), List
	`name`: String
	`autospawn`: Boolean
	`labels`: String, List
	`node_type`: [NodeTypeConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/node/node_factory/enum.NodeTypeConfig.html), Enum
		- `type`: Robot => [RobotConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/node/node_factory/struct.RobotConfig.html)
			`navigator`: [NavigatorConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/navigators/mod/enum.NavigatorConfig.html), Enum
				- `type`: TrajectoryFollower => [TrajectoryFollowerConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/navigators/trajectory_follower/struct.TrajectoryFollowerConfig.html)
					`trajectory`: [TrajectoryConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/navigators/trajectory/struct.TrajectoryConfig.html)
						`point_list`: Float, List, List
						`do_loop`: Boolean
					`forward_distance`: Float
					`target_speed`: Float
					`stop_distance`: Float
					`stop_ramp_coefficient`: Float
				- `type`: Plugin => GeneralValue
				- `type`: GoTo => [GoToConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/navigators/go_to/struct.GoToConfig.html)
					`target_point`: f32, Optional, Array\[2\]
					`target_speed`: Float
					`stop_distance`: Float
					`stop_ramp_coefficient`: Float
			`controller`: [ControllerConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/controllers/mod/enum.ControllerConfig.html), Enum
				- `type`: PID => [PIDConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/controllers/pid/struct.PIDConfig.html)
					`robot_model`: [RobotModelConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/physics/robot_models/mod/enum.RobotModelConfig.html), Optional, Enum
						- `type`: Unicycle => [UnicycleConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/physics/robot_models/unicycle/struct.UnicycleConfig.html)
							`wheel_distance`: Float
						- `type`: Holonomic => [HolonomicConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/physics/robot_models/holonomic/struct.HolonomicConfig.html)
							`max_longitudinal_velocity`: Float
							`max_lateral_velocity`: Float
							`max_angular_velocity`: Float
					`proportional_gains`: Float, List
					`derivative_gains`: Float, List
					`integral_gains`: Float, List
				- `type`: Plugin => GeneralValue
			`physics`: [PhysicsConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/physics/mod/enum.PhysicsConfig.html), Enum
				- `type`: Internal => [InternalPhysicConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/physics/internal_physics/struct.InternalPhysicConfig.html)
					`model`: [RobotModelConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/physics/robot_models/mod/enum.RobotModelConfig.html), See above
					`initial_state`: [StateConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/state_estimators/mod/struct.StateConfig.html)
						`pose`: Float, List
						`velocity`: Float, List
						`random`: [RandomVariableTypeConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/utils/determinist_random_variable/enum.RandomVariableTypeConfig.html), List, Enum
							- `type`: None  
							- `type`: Fixed => [FixedRandomVariableConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/utils/distributions/fixed/struct.FixedRandomVariableConfig.html)
								`values`: Float, List
							- `type`: Uniform => [UniformRandomVariableConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/utils/distributions/uniform/struct.UniformRandomVariableConfig.html)
								`min`: Float, List
								`max`: Float, List
							- `type`: Normal => [NormalRandomVariableConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/utils/distributions/normal/struct.NormalRandomVariableConfig.html)
								`mean`: Float, List
								`covariance`: Float, List
							- `type`: Poisson => [PoissonRandomVariableConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/utils/distributions/poisson/struct.PoissonRandomVariableConfig.html)
								`lambda`: Float, List
							- `type`: Exponential => [ExponentialRandomVariableConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/utils/distributions/exponential/struct.ExponentialRandomVariableConfig.html)
								`lambda`: Float, List
						`variable_order`: String, List
					`faults`: [PhysicsFaultModelConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/physics/fault_models/mod/enum.PhysicsFaultModelConfig.html), List, Enum
						- `type`: AdditiveRobotCentered => [AdditiveRobotCenteredPhysicsFaultConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/physics/fault_models/additive_robot_centered/struct.AdditiveRobotCenteredPhysicsFaultConfig.html)
							`distributions`: [RandomVariableTypeConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/utils/determinist_random_variable/enum.RandomVariableTypeConfig.html), See above, List
							`variable_order`: PhysicsVariables, List
							`proportional_to`: PhysicsVariables, Optional
							`proportional_factor`: Float, Optional
						- `type`: Plugin => GeneralValue
				- `type`: Plugin => GeneralValue
			`state_estimator`: [StateEstimatorConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/state_estimators/mod/enum.StateEstimatorConfig.html), Enum
				- `type`: Perfect => [PerfectEstimatorConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/state_estimators/perfect_estimator/struct.PerfectEstimatorConfig.html)
					`prediction_activation`: [PeriodicityConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/utils/periodicity/struct.PeriodicityConfig.html), Optional
						`period`: [NumberConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/config/enum.NumberConfig.html), Enum
							- `type`: Num => Float
							- `type`: Rand => [RandomVariableTypeConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/utils/determinist_random_variable/enum.RandomVariableTypeConfig.html), See above
						`offset`: [NumberConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/config/enum.NumberConfig.html), See above, Optional
						`table`: Float, Optional, List
					`targets`: String, List
				- `type`: Plugin => GeneralValue
			`sensor_manager`: [SensorManagerConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/sensor_manager/struct.SensorManagerConfig.html)
				`sensors`: [ManagedSensorConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/sensor_manager/struct.ManagedSensorConfig.html), List
					`name`: String
					`send_to`: String, List
					`triggered`: Boolean
					`config`: [SensorConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/mod/enum.SensorConfig.html), Enum
						- `type`: OrientedLandmark => [OrientedLandmarkSensorConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/oriented_landmark_sensor/struct.OrientedLandmarkSensorConfig.html)
							`detection_distance`: Float
							`activation_time`: [PeriodicityConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/utils/periodicity/struct.PeriodicityConfig.html), See above, Optional
							`faults`: [OrientedLandmarkSensorFaultModelConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/oriented_landmark_sensor/enum.OrientedLandmarkSensorFaultModelConfig.html), List, Enum
								- `type`: AdditiveRobotCentered => [AdditiveFaultConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/fault_models/additive/struct.AdditiveFaultConfig.html)
									`apparition`: [BernouilliRandomVariableConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/utils/distributions/bernouilli/struct.BernouilliRandomVariableConfig.html)
										`probability`: Float, List
									`distributions`: [RandomVariableTypeConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/utils/determinist_random_variable/enum.RandomVariableTypeConfig.html), See above, List
									`variable_order`: SVO, List
									`proportional_to`: SVProp, Optional
									`proportional_factor`: Float, Optional
								- `type`: AdditiveObservationCentered => [AdditiveFaultConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/fault_models/additive/struct.AdditiveFaultConfig.html), See above
								- `type`: Clutter => [ClutterFaultConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/fault_models/clutter/struct.ClutterFaultConfig.html)
									`apparition`: [RandomVariableTypeConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/utils/determinist_random_variable/enum.RandomVariableTypeConfig.html), See above
									`distributions`: [RandomVariableTypeConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/utils/determinist_random_variable/enum.RandomVariableTypeConfig.html), See above, List
									`variable_order`: SV, List
									`observation_id`: String
								- `type`: Misdetection => [MisdetectionFaultConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/fault_models/misdetection/struct.MisdetectionFaultConfig.html)
									`apparition`: [BernouilliRandomVariableConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/utils/distributions/bernouilli/struct.BernouilliRandomVariableConfig.html), See above
								- `type`: Misassociation => [MisassociationFaultConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/fault_models/misassociation/struct.MisassociationFaultConfig.html)
									`apparition`: [BernouilliRandomVariableConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/utils/distributions/bernouilli/struct.BernouilliRandomVariableConfig.html), See above
									`distribution`: [RandomVariableTypeConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/utils/determinist_random_variable/enum.RandomVariableTypeConfig.html), See above
									`sort`: [Sort](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/fault_models/misassociation/enum.Sort.html), Enum
										- `type`: None  
										- `type`: Random  
										- `type`: Distance  
									`source`: [Source](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/fault_models/misassociation/enum.Source.html), Enum
										- `type`: Map  
										- `type`: Robots  
								- `type`: Plugin => GeneralValue
							`filters`: [OrientedLandmarkSensorFilterConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/oriented_landmark_sensor/enum.OrientedLandmarkSensorFilterConfig.html), List, Enum
								- `type`: Range => [RangeFilterConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/sensor_filters/range_filter/struct.RangeFilterConfig.html)
									`variables`: SV, List
									`min_range`: Float, List
									`max_range`: Float, List
									`inside`: Boolean
								- `type`: Id => [StringFilterConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/sensor_filters/string_filter/struct.StringFilterConfig.html)
									`accepted`: String, List
									`rejected`: String, List
									`priority_accept`: Boolean
								- `type`: Label => [StringFilterConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/sensor_filters/string_filter/struct.StringFilterConfig.html), See above
								- `type`: Plugin => GeneralValue
							`xray`: Boolean
						- `type`: Speed => [SpeedSensorConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/speed_sensor/struct.SpeedSensorConfig.html)
							`activation_time`: [PeriodicityConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/utils/periodicity/struct.PeriodicityConfig.html), See above, Optional
							`faults`: [SpeedSensorFaultModelConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/speed_sensor/enum.SpeedSensorFaultModelConfig.html), List, Enum
								- `type`: Additive => [AdditiveFaultConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/fault_models/additive/struct.AdditiveFaultConfig.html), See above
								- `type`: Plugin => GeneralValue
							`filters`: [SpeedSensorFilterConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/speed_sensor/enum.SpeedSensorFilterConfig.html), List, Enum
								- `type`: Range => [RangeFilterConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/sensor_filters/range_filter/struct.RangeFilterConfig.html), See above
								- `type`: Plugin => GeneralValue
						- `type`: Displacement => [DisplacementSensorConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/displacement_sensor/struct.DisplacementSensorConfig.html)
							`activation_time`: [PeriodicityConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/utils/periodicity/struct.PeriodicityConfig.html), See above, Optional
							`faults`: [DisplacementSensorFaultModelConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/displacement_sensor/enum.DisplacementSensorFaultModelConfig.html), List, Enum
								- `type`: AdditivePreDisplacement => [AdditiveFaultConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/fault_models/additive/struct.AdditiveFaultConfig.html), See above
								- `type`: AdditivePostDisplacement => [AdditiveFaultConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/fault_models/additive/struct.AdditiveFaultConfig.html), See above
								- `type`: Plugin => GeneralValue
							`filters`: [DisplacementSensorFilterConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/displacement_sensor/enum.DisplacementSensorFilterConfig.html), List, Enum
								- `type`: Range => [RangeFilterConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/sensor_filters/range_filter/struct.RangeFilterConfig.html), See above
								- `type`: Plugin => GeneralValue
							`lie_movement`: Boolean
						- `type`: GNSS => [GNSSSensorConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/gnss_sensor/struct.GNSSSensorConfig.html)
							`activation_time`: [PeriodicityConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/utils/periodicity/struct.PeriodicityConfig.html), See above, Optional
							`faults`: [GNSSSensorFaultModelConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/gnss_sensor/enum.GNSSSensorFaultModelConfig.html), List, Enum
								- `type`: Additive => [AdditiveFaultConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/fault_models/additive/struct.AdditiveFaultConfig.html), See above
								- `type`: Clutter => [ClutterFaultConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/fault_models/clutter/struct.ClutterFaultConfig.html), See above
								- `type`: Misdetection => [MisdetectionFaultConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/fault_models/misdetection/struct.MisdetectionFaultConfig.html), See above
								- `type`: Plugin => GeneralValue
							`filters`: [GNSSSensorFilterConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/gnss_sensor/enum.GNSSSensorFilterConfig.html), List, Enum
								- `type`: Range => [RangeFilterConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/sensor_filters/range_filter/struct.RangeFilterConfig.html), See above
								- `type`: Plugin => GeneralValue
						- `type`: Robot => [RobotSensorConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/robot_sensor/struct.RobotSensorConfig.html)
							`detection_distance`: Float
							`activation_time`: [PeriodicityConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/utils/periodicity/struct.PeriodicityConfig.html), See above, Optional
							`faults`: [RobotSensorFaultModelConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/robot_sensor/enum.RobotSensorFaultModelConfig.html), List, Enum
								- `type`: AdditiveRobotCentered => [AdditiveFaultConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/fault_models/additive/struct.AdditiveFaultConfig.html), See above
								- `type`: AdditiveObservationCentered => [AdditiveFaultConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/fault_models/additive/struct.AdditiveFaultConfig.html), See above
								- `type`: Clutter => [ClutterFaultConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/fault_models/clutter/struct.ClutterFaultConfig.html), See above
								- `type`: Misdetection => [MisdetectionFaultConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/fault_models/misdetection/struct.MisdetectionFaultConfig.html), See above
								- `type`: Misassociation => [MisassociationFaultConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/fault_models/misassociation/struct.MisassociationFaultConfig.html), See above
								- `type`: Plugin => GeneralValue
							`filters`: [RobotSensorFilterConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/robot_sensor/enum.RobotSensorFilterConfig.html), List, Enum
								- `type`: Range => [RangeFilterConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/sensor_filters/range_filter/struct.RangeFilterConfig.html), See above
								- `type`: Id => [StringFilterConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/sensor_filters/string_filter/struct.StringFilterConfig.html), See above
								- `type`: Label => [StringFilterConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/sensor_filters/string_filter/struct.StringFilterConfig.html), See above
								- `type`: Plugin => GeneralValue
							`xray`: Boolean
						- `type`: Scan => [ScanSensorConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/scan_sensor/struct.ScanSensorConfig.html)
							`detection_distance`: Float
							`rays`: [RayConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/scan_sensor/enum.RayConfig.html), Enum
								- `type`: Regular => usize
								- `type`: DegreeTable => Float, List
								- `type`: RadianTable => Float, List
							`height`: Float
							`activation_time`: [PeriodicityConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/utils/periodicity/struct.PeriodicityConfig.html), See above, Optional
							`faults`: [ScanSensorFaultModelConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/scan_sensor/enum.ScanSensorFaultModelConfig.html), List, Enum
								- `type`: AdditiveRobotCentered => [AdditiveFaultConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/fault_models/additive/struct.AdditiveFaultConfig.html), See above
								- `type`: PointAdditiveRobotCentered => [AdditiveFaultConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/fault_models/additive/struct.AdditiveFaultConfig.html), See above
								- `type`: PointAdditiveObservationCentered => [AdditiveFaultConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/fault_models/additive/struct.AdditiveFaultConfig.html), See above
								- `type`: Clutter => [ClutterFaultConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/fault_models/clutter/struct.ClutterFaultConfig.html), See above
								- `type`: Misdetection => [MisdetectionFaultConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/fault_models/misdetection/struct.MisdetectionFaultConfig.html), See above
								- `type`: PointMisdetection => [MisdetectionFaultConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/fault_models/misdetection/struct.MisdetectionFaultConfig.html), See above
								- `type`: Plugin => GeneralValue
							`filters`: [ScanSensorFilterConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/scan_sensor/enum.ScanSensorFilterConfig.html), List, Enum
								- `type`: Range => [RangeFilterConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/sensors/sensor_filters/range_filter/struct.RangeFilterConfig.html), See above
								- `type`: Plugin => GeneralValue
						- `type`: Plugin => GeneralValue
					`frame`: [FrameConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/utils/frame/struct.FrameConfig.html)
						`x`: Float
						`y`: Float
						`theta`: Float
			`network`: [NetworkConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/networking/network/struct.NetworkConfig.html)
				`range`: Float
				`reception_delay`: Float
			`state_estimator_bench`: [BenchStateEstimatorConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/state_estimators/mod/struct.BenchStateEstimatorConfig.html), List
				`name`: String
				`config`: [StateEstimatorConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/state_estimators/mod/enum.StateEstimatorConfig.html), See above
		- `type`: ComputationUnit => [ComputationUnitConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/node/node_factory/struct.ComputationUnitConfig.html)
			`network`: [NetworkConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/networking/network/struct.NetworkConfig.html), See above
			`state_estimators`: [BenchStateEstimatorConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/state_estimators/mod/struct.BenchStateEstimatorConfig.html), See above, List
`scenario`: [ScenarioConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/scenario/config/struct.ScenarioConfig.html)
	`events`: [EventConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/scenario/config/struct.EventConfig.html), List
		`triggering_nodes`: String, List
		`trigger`: [EventTriggerConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/scenario/config/enum.EventTriggerConfig.html), Enum
			- `type`: Time => [TimeEventTriggerConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/scenario/config/struct.TimeEventTriggerConfig.html)
				`time`: [NumberConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/config/enum.NumberConfig.html), See above
				`occurences`: [NumberConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/config/enum.NumberConfig.html), See above
			- `type`: Proximity => [ProximityEventTriggerConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/scenario/config/struct.ProximityEventTriggerConfig.html)
				`protected_target`: String, Optional
				`distance`: Float
				`inside`: Boolean
			- `type`: Area => [AreaEventTriggerConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/scenario/config/enum.AreaEventTriggerConfig.html), Enum
				- `type`: Rect => [RectAreaEventTriggerConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/scenario/config/struct.RectAreaEventTriggerConfig.html)
					`bottom_left`: )
					`top_right`: )
					`inside`: Boolean
				- `type`: Circle => [CircleAreaEventTriggerConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/scenario/config/struct.CircleAreaEventTriggerConfig.html)
					`center`: )
					`radius`: Float
					`inside`: Boolean
		`event_type`: [EventTypeConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/scenario/config/enum.EventTypeConfig.html), Enum
			- `type`: Spawn => [SpawnEventConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/scenario/config/struct.SpawnEventConfig.html)
				`model_name`: String
				`node_name`: String
			- `type`: Kill => String
			- `type`: SendMessage => [SendMessageEventConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/scenario/config/struct.SendMessageEventConfig.html)
				`channel`: String
				`message`: MessageType
`environment`: [EnvironmentConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/environment/mod/struct.EnvironmentConfig.html)
	`map`: [MapConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/environment/map/struct.MapConfig.html)
		`landmarks`: [OrientedLandmarkConfig](https://homepages.laas.fr/mescourrou/Recherche/Logiciels/multi-robot-simulator/rust/simusa/environment/oriented_landmark/struct.OrientedLandmarkConfig.html), List
			`id`: Integer
			`labels`: String, List
			`x`: Float
			`y`: Float
			`theta`: Float
			`height`: Float
			`width`: Float
		`random`: RandomMap, List
