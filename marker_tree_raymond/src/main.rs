extern crate mpi;

use mpi::topology::SystemCommunicator;
use mpi::traits::*;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use std::process;
use std::thread;
use std::time::Duration;

// Идентификатор запроса
// - если в процессе есть МАРКЕР, то он может попасть в критическую секцию
// - иначе:
//   - поместить запрос в очередь
//   - послать ЗАПРОС в направлении маркера
enum SIGNAL {
    Marker,
    Request,
}

// Вспомогательная инструкция для вывода информации об элементе дерева процессов
#[derive(Debug)]
struct ProcTree {
    queue: Vec<i32>, // Очередь запросов в текущем элементе дерева
    rk: i32,         // Идентификатор процесса - RANK(MPI)
    root: i32,       // Идентификатор "родителя" в древовидной схеме процессов
    // - номер идентификатора родительского процесса - RANK(MPI)
    left: i32, // Идентификатор "левого сына" в древовидной схеме процессов
    // - RANK(MPI)
    right: i32, // Идентификатор "правого сына" в древовидной схеме процессов
    // - RANK(MPI)
    to_proc: i32, // Указатель направления маркера:
    // - 0 - parent,
    // - 1 - left child,
    // - 2 - right child
    marker: bool, // Идентификатор наличия маркера для доступа к КС
}

// Реализация основных методов дерева процессов
// - receive - обработка сообщений(запроса) МАРКЕР и ЗАПРОС
// - critical - симуляция критическй секции
impl ProcTree {
    // 2 вида сообщений(см enum SIGNAL):
    // - SIGNAL::Marker:
    //      - Взять 1-ый запрос из очереди и
    //        послать маркер автору(если автор сам процесс, то попасть в КС)
    //      - Поменять значение указателя в сторону маркера
    //      - Иключение запроса из очереди
    //      - Если очередь не пуста, отправить ЗАПРОС в направлении маркера
    // - SIGNAL::Request:
    //      - Добавить запрос в очередь
    //      - Маркер есть в процессе?
    //          - ДА => отправить себе сообщение МАРКЕР
    //          - НЕТ => отправить ЗАПРОС в направлении маркера
    fn receive(
        &mut self,
        signal: SIGNAL,
        world: SystemCommunicator,
        sender: i32,
        iter: i32,
        size: i32,
    ) {
        match signal {
            SIGNAL::Marker => {
                // В случае, если очередь не пуста, то извлекается последний элемент
                if let Some(first_in_queue) = self.queue.pop() {
                    println!(
                        "Extracted {}. Stack of {} is {:?}",
                        first_in_queue, self.rk, &self.queue
                    );

                    // Отправка маркера автору
                    match first_in_queue {
                        _ if self.rk == first_in_queue => {
                            // Если автор сам процесс, то допуск к КС
                            self.marker = true;
                            ProcTree::critical(self)
                        }
                        // Отправка маркера в направлении автора и обновление параметров дерева
                        rooter if self.root == first_in_queue => {
                            println!("SEND TO ROOT {} FROM {}", rooter, self.rk);
                            self.marker = false;
                            self.to_proc = 0;
                            let fname: String = self.rk.to_string() + ".txt";
                            let mut file = OpenOptions::new()
                                .write(true)
                                .append(true)
                                .open(fname)
                                .unwrap();

                            file.write_all(b"Send Marker to root\n")
                                .expect("Error logging info");
                            world.process_at_rank(rooter).send(&0)
                        }
                        left if self.left == first_in_queue => {
                            println!("SEND TO CHILD {} FROM {}", left, self.rk);
                            self.marker = false;
                            self.to_proc = 1;
                            let fname: String = self.rk.to_string() + ".txt";
                            let mut file = OpenOptions::new()
                                .write(true)
                                .append(true)
                                .open(fname)
                                .unwrap();

                            file.write_all(b"Send Marker to left child\n")
                                .expect("Error logging info");
                            world.process_at_rank(left).send(&0)
                        }
                        right if self.right == first_in_queue => {
                            println!("SEND TO CHILD {} FROM {}", right, self.rk);
                            self.marker = false;
                            self.to_proc = 2;
                            let fname: String = self.rk.to_string() + ".txt";
                            let mut file = OpenOptions::new()
                                .write(true)
                                .append(true)
                                .open(fname)
                                .unwrap();

                            file.write_all(b"Send Marker to right child\n")
                                .expect("Error logging info");
                            world.process_at_rank(right).send(&0)
                        }
                        // Неверно задан автор маркера(не связан с элементом дерева)
                        _ => println!("Invalid rank in queue {}", first_in_queue),
                    }
                    // Если очередь с запросами, то отправить ЗАПРОС в направлении маркера
                    if let Some(top) = self.queue.pop() {
                        ProcTree::receive(self, SIGNAL::Request, world, top, iter, size);
                    }
                }
            }
            SIGNAL::Request => {
                // Добавить запрос в очередь(аргумент из параметров сообщения - sender)
                self.queue.push(sender);
                // Служебная информация о текущем состоянии очереди запросов
                println!("Stack of {} is {:?}", self.rk, &self.queue);
                if self.marker {
                    // Если маркер есть в процессе, отправляем сообщение MARKER себе же
                    ProcTree::receive(self, SIGNAL::Marker, world, self.rk, iter, size);
                } else {
                    // Иначе отправляем ЗАПРОС в направлении маркера
                    match self.to_proc {
                        0 => {
                            let fname: String = self.rk.to_string() + ".txt";
                            let mut file = OpenOptions::new()
                                .write(true)
                                .append(true)
                                .open(fname)
                                .unwrap();

                            file.write_all(b"Send Recieve to root\n")
                                .expect("Error logging info");
                            world.process_at_rank(self.root).send(&1)
                        }
                        1 => {
                            let fname: String = self.rk.to_string() + ".txt";
                            let mut file = OpenOptions::new()
                                .write(true)
                                .append(true)
                                .open(fname)
                                .unwrap();

                            file.write_all(b"Send Recieve to left child\n")
                                .expect("Error logging info");
                            world.process_at_rank(self.left).send(&1)
                        }
                        2 => {
                            let fname: String = self.rk.to_string() + ".txt";
                            let mut file = OpenOptions::new()
                                .write(true)
                                .append(true)
                                .open(fname)
                                .unwrap();

                            file.write_all(b"Send Recieve to right child\n")
                                .expect("Error logging info");
                            world.process_at_rank(self.right).send(&1)
                        }
                        // Неправильный параметр to_proc(направление маркера)
                        _ => println!("Invalid to_proc field {}", self.to_proc),
                    }
                }
            }
        }
    }
    // Симуляция критической секции:
    // - Если файл critical.txt существует, то
    //   аварийно завершить программу
    //   (одновременно >1 процесса пытаются получить доступ в критическую секцию)
    // - Иначе создать файл и "заснуть" на 5 секунд, затем удалить файл
    fn critical(&mut self) {
        let result = match File::open("critical.txt") {
            Ok(_) => {
                println!("Accessed to critical section without permission");
                -1
            }
            Err(_) => 0,
        };

        if result == 0 {
            let fname: String = self.rk.to_string() + ".txt";
            let mut file = OpenOptions::new()
                .write(true)
                .append(true)
                .open(fname)
                .unwrap();
            file.write_all(b"Accesed critical section\n")
                .expect("Error writing");
            File::create("critical.txt").expect("Error creating critical.txt");
            thread::sleep(Duration::from_secs(1));
            fs::remove_file("critical.txt").expect("Error deleting critical.txt");
        } else {
            println!("Critical file exsisted! ERROR! Terminating...");
            process::abort();
        }
    }
}

fn main() {
    // Стандартная инициаллизация MPI
    let universe = mpi::initialize().unwrap();
    let world = universe.world();
    let size = world.size();
    let rank = world.rank();

    // Корнем дерева процесса считается 0-ой процесс(структура дерева - КУЧА)
    let tree_node_proc = 0;
    // Аргумент командной строки указывает, где хранится маркер
    let args: Vec<String> = env::args().collect();
    let marker_node: i32 = match {
        if args.get(1).is_none() {
            String::new()
        } else {
            args[1].clone()
        }
    }
    .trim()
    .parse::<i32>()
    {
        Ok(num) => num,
        Err(_) => {
            // Если ничего не указано в командной строке,
            // то маркер расположим на последнем процессе
            if rank == tree_node_proc {
                println!(
                    "No number entered, so marker will be at process with rank={}.",
                    size - 1
                );
            }
            size - 1
        }
    };

    // Создаем служебную переменную и словарь,
    // чтобы построить правильный маршрут до маркера
    let mut marker_cnt = marker_node.clone();
    let mut child_to_path = HashMap::new();

    while marker_cnt != tree_node_proc {
        child_to_path.insert((marker_cnt - 1) / 2, marker_cnt);
        marker_cnt = (marker_cnt - 1) / 2;
    }

    // Инициаллизируем дерево процессов
    // (
    //      если ссылка на элемент не может быть добавлена:
    //          - ранг меньше 0;
    //          - ранг больше size
    //      , то указываем -1
    // )
    let mut tree_elem = match rank {
        0 => ProcTree {
            queue: Vec::new(),
            rk: tree_node_proc,
            root: -1,
            left: 1,
            right: 2,
            to_proc: 1,
            marker: marker_node == tree_node_proc,
        },
        num => {
            let mut left = 2 * num + 1;
            if left >= size {
                left = -1;
            }

            let mut right = 2 * num + 2;
            if right >= size {
                right = -1;
            }

            ProcTree {
                queue: Vec::new(),
                rk: num,
                root: (num - 1) / 2,
                left,
                right,
                to_proc: 0,
                marker: marker_node == num,
            }
        }
    };

    // Установим корректный маршрут до маркера из всех элементов дерева
    if rank == 0 {
        let fname: String = rank.to_string() + ".txt";
        File::create(fname).expect("Error rank creation file");
        if tree_elem.left != -1 {
            world
                .process_at_rank(tree_elem.left)
                .send(&tree_elem.marker);
        }
        if tree_elem.right != -1 {
            world
                .process_at_rank(tree_elem.right)
                .send(&tree_elem.marker);
        }
        let relative_marker_path = child_to_path[&rank];

        if tree_elem.left != -1 && relative_marker_path == tree_elem.left {
            tree_elem.to_proc = 1;
        } else if tree_elem.right != -1 && relative_marker_path == tree_elem.right {
            tree_elem.to_proc = 2;
        }
    }
    for i in 1..size {
        if rank == i {
            let fname: String = rank.to_string() + ".txt";
            File::create(fname).expect("Error rank creation file");

            world.process_at_rank(tree_elem.root).receive::<bool>();

            if child_to_path.contains_key(&rank) {
                let relative_marker_path = child_to_path[&rank];

                if tree_elem.left != -1 && relative_marker_path == tree_elem.left {
                    tree_elem.to_proc = 1;
                } else if tree_elem.right != -1 && relative_marker_path == tree_elem.right {
                    tree_elem.to_proc = 2;
                }
            }

            if tree_elem.left != -1 {
                world
                    .process_at_rank(tree_elem.left)
                    .send(&tree_elem.marker);
            }
            if tree_elem.right != -1 {
                world
                    .process_at_rank(tree_elem.right)
                    .send(&tree_elem.marker);
            }
        }
    }

    let mut previous_marker: i32 = marker_node;
    for request_sender in 0..size {
        //Ожидаем все процессы после окончания предыдущей итерации цикла для начала новой
        world.barrier();

        // Логируем информацию об итерациях
        let fname: String = tree_elem.rk.to_string() + ".txt";
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(fname)
            .unwrap();
        if request_sender > 0 {
            file.write_all(b"####################\n")
                .expect("Error logging info");
        }

        let log_info = format!(
            "####################\nIteration {}/{}\n",
            request_sender + 1,
            size
        );
        file.write_all(log_info.as_bytes())
            .expect("Error logging info");

        // Напечатаем все дерево от корня к листам через send, receive
        if rank == 0 {
            println!(
                "\n###############################################
                Iteration {}/{}. Process tree is:",
                request_sender + 1,
                size
            );
            if tree_elem.left != -1 {
                world
                    .process_at_rank(tree_elem.left)
                    .send(&tree_elem.marker);
            }
            if tree_elem.right != -1 {
                world
                    .process_at_rank(tree_elem.right)
                    .send(&tree_elem.marker);
            }
            println!("{:?}", tree_elem);
        }
        for i in 1..size {
            if rank == i {
                world.process_at_rank(tree_elem.root).receive::<bool>();

                println!("{:?}", tree_elem);

                if tree_elem.left != -1 {
                    world
                        .process_at_rank(tree_elem.left)
                        .send(&tree_elem.marker);
                }
                if tree_elem.right != -1 {
                    world
                        .process_at_rank(tree_elem.right)
                        .send(&tree_elem.marker);
                }
            }
        }

        // Ожидаем все процессы перед работай маркерного алгоритма
        world.barrier();

        // Основной процесс-запрос - rank=request_sender, поэтому от него отправляем Request
        if rank == request_sender {
            println!("Node with rank {} wants to enter the CRITICAL SECTION. Marker owner is node with rank {}.", rank, previous_marker);
            tree_elem.receive(SIGNAL::Request, world, rank, request_sender, size);
            let (idx, _) = match tree_elem.to_proc {
                0 if tree_elem.to_proc == 0 => {
                    world.process_at_rank(tree_elem.root).receive::<i32>()
                }
                1 if tree_elem.to_proc == 1 => {
                    world.process_at_rank(tree_elem.left).receive::<i32>()
                }
                2 if tree_elem.to_proc == 2 => {
                    world.process_at_rank(tree_elem.right).receive::<i32>()
                }
                _ => {
                    println!("Invalid MARKER DIRECTION AT NODE {}", rank);
                    process::abort();
                }
            };

            // idx - информация о типе запроса - либо Marker(idx == 0), либо Request(idx == 0)
            match idx {
                0 => tree_elem.receive(SIGNAL::Marker, world, tree_elem.root, request_sender, size),
                1 => {
                    tree_elem.receive(SIGNAL::Request, world, tree_elem.root, request_sender, size)
                }
                _ => println!("Invalid msg!"),
            }
            // Завершение работы программы и отправка всем процессам специального сигнала
            for i in 0..size {
                if i != request_sender {
                    world.process_at_rank(i).send(&2);
                }
            }
        } else {
            // Остальные процессы непрерывно слушают запросы(единственный признак остановки - специальный сигнал inp==2)
            let mut inp = 1;
            while inp != 2 {
                let (inp1, status) = world.any_process().receive::<i32>();
                inp = inp1;
                let source: i32 = status.source_rank();
                match inp {
                    0 => tree_elem.receive(SIGNAL::Marker, world, source, request_sender, size),
                    1 => tree_elem.receive(SIGNAL::Request, world, source, request_sender, size),
                    2 => {
                        //Завершение работы процесса
                    }
                    _ => println!("Invalid msg!"),
                }
            }
        }
        previous_marker = request_sender;

        // Ожидаем окончания работы процессов, выводим окончание итерации
        world.barrier();
        if rank == 0 {
            println!("###############################################");
        }
    }

    // Конец логирования
    let fname: String = tree_elem.rk.to_string() + ".txt";
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(fname)
        .unwrap();

    file.write_all(b"####################\n")
        .expect("Error logging info");
    world.barrier();
}
