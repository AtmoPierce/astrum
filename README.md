# Astrum – Parallelized Model Predictive Control

Astrum is a research framework for **Model Predictive Control (MPC)** with emphasis on **parallelized multiple shooting** methods.  
The project explores CPU and GPU acceleration strategies for large-scale optimal control, comparing parallelization methods across threads, processes, and devices.

> This work is being developed as part of my **graduate thesis in Software Engineering**,  
> focusing on the intersection of high-performance computing and control systems.

---

## Capability Checklist

### Core MPC
- [ ] Multiple shooting problem formulation  
- [ ] Linear time-varying system dynamics  
- [ ] Quadratic cost with constraints  
- [ ] Discrete Riccati recursion (DRE/DARE solver)  
- [ ] Control gain update (`K = R⁻¹ Bᵀ P`)  

### Parallelization Methods
- [ ] **Sequential baseline**: single-thread CPU shooting  
- [ ] **Multithreaded CPU**: thread-level parallelism with work-sharing  
- [ ] **GPU batched**: CUDA/cuBLAS for batched Riccati solves  
- [ ] **Hybrid**: multi-CPU with GPU acceleration for inner loops  
- [ ] **Pipeline parallelism**: overlapping dynamics propagation and Riccati steps  

### Solver Infrastructure
- [ ] Batched state propagation (position, velocity, attitude)  
- [ ] Parallel integration (trapezoidal / RK4) across horizons  
- [ ] Sparse KKT system assembly  
- [ ] Comparison of direct vs iterative solvers  
- [ ] Configurable horizon length, batch size, and dynamics models  

### Performance & Evaluation
- [ ] Benchmark harness (timing, scaling with horizon length & batch size)  
- [ ] Comparison plots: CPU sequential vs multithread vs GPU  
- [ ] Strong scaling (fixed problem, more cores/SMs)  
- [ ] Weak scaling (growing horizon size)  
- [ ] Accuracy verification against sequential baseline  

### Extensions
- [ ] Nonlinear dynamics linearization per shooting node  
- [ ] Warm-start strategies across MPC iterations  
- [ ] Support for constraints (input/state bounds)  
- [ ] Comparison with direct collocation methods  
- [ ] Integration with visualization (Telegraph)  

---

## Project Scope

This project investigates **how different parallelization methods affect the runtime of MPC with multiple shooting**:

- **Multithreaded CPU** → shared memory parallelization  
- **GPU (CUDA/cuBLAS)** → batched Riccati and integration steps  
- **Hybrid scheduling** → using both CPU threads and GPU kernels efficiently  

The ultimate goal is to **map the algorithm to the hardware** and identify trade-offs between:
- Work granularity  
- Communication overhead  
- Numerical stability  
- Implementation complexity  

---

## Status

- [ ] Baseline MPC with sequential multiple shooting  
- [ ] Batched Riccati solver on GPU  
- [ ] CPU multithreaded integration  
- [ ] Parallelization comparison experiments  
