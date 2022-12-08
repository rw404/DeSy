extern crate mpi;

use mpi::collective::SystemOperation;
use mpi::traits::*;
use std::env;

fn main() {
    let universe = mpi::initialize().unwrap();
    let world = universe.world();
    let size = world.size();
    let rank = world.rank();
    //let squared_size: usize = 4;
    let mut local_rank = 0;
    let mut global_rank = 0;
    let args: Vec<String> = env::args().collect();

    // Select root thread
    let root_rank = 0;
    let root_process = world.process_at_rank(root_rank);

    let squared_size: usize = match {
        if args.get(1).is_none() {
            String::new()
        } else {
            args[1].clone()
        }
    }
    .trim()
    .parse::<usize>()
    {
        Ok(num) => num,
        Err(_) => {
            if rank == root_rank {
                println!("No number entered, so matrix will be 4x4");
            }
            4 as usize
        }
    };

    // Create and init shared matrix
    let mut arr;
    if world.rank() == root_rank {
        arr = vec![0.0; squared_size * squared_size];
        for row_idx in 0..squared_size {
            arr[row_idx * squared_size] = row_idx as f64;
            arr[row_idx * squared_size + row_idx] = 1.0;
        }
        println!("TOTAL PROCS {}", size);
        if squared_size < 10 {
            println!("Root owns matrix:");
            for i in 0..4 {
                println!("{:?}", &arr[4 * i..4 * i + 4]);
            }
        } else {
            println!(
                "Matrix of size {} is large, so it wouldn't be printed",
                squared_size
            );
        }
        println!(
            "GAUSS {}x{}\n------------------------------",
            squared_size, squared_size,
        );
    } else {
        arr = vec![0.0; squared_size * squared_size];
    }
    root_process.broadcast_into(&mut arr[..]);

    // Construct row mapper
    let mut row_mapper: Vec<usize> = vec![0; squared_size];
    for (idx, el) in &mut row_mapper.iter_mut().enumerate() {
        *el = idx % (size as usize);
    }

    let proc_mapper: Vec<usize> = row_mapper;

    if rank == root_rank {
        println!("ADDITIONAL INFO\n##############################");
        println!("Matrix row distribution map:");

        print!("\tRows:");
        for i in 0..(squared_size as usize) {
            print!("  {}", i);
        }
        println!("");
        println!("\tProcs: {:?}", &proc_mapper);
        println!("##############################");
    }

    let t_start = mpi::time();
    for i in 0..squared_size {
        let sender_process = world.process_at_rank(proc_mapper[i] as i32);
        sender_process.broadcast_into(&mut arr[i * squared_size + i..(i + 1) * squared_size]);
        if proc_mapper[i] == rank as usize {
            if arr[i * squared_size + i] != 0.0 {
                arr[i * squared_size + i] = 1.0;
                local_rank += 1;
            } else {
                break;
            }
        }

        for k in i + 1..squared_size {
            if proc_mapper[k] == rank as usize {
                let mut divider = arr[i * squared_size + i].clone();
                if divider == 0.0 {
                    divider = 1.0;
                }
                for j in i + 1..squared_size {
                    arr[k * squared_size + j] -=
                        arr[k * squared_size + i] * arr[i * squared_size + j] / divider;
                }
                arr[k * squared_size + i] = 0.0;
            }
        }
    }

    world.all_reduce_into(&local_rank, &mut global_rank, SystemOperation::sum());
    world.barrier();
    let t_end = mpi::time();

    if rank == root_rank {
        println!("RESULT\n------------------------------");
        println!("Rank is {}", global_rank);
        println!("Estimated time is {} seconds", t_end - t_start);
    }
}
