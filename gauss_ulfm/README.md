# <center>Отчет по курсу <br>"Распределенные системы"</center>

<center>Создание отказоустойчивой параллельной версии программы вычисления ранга матрицы эквивалеными преобразованиями к ступенчатому виду.</center>
---

<div style="text-align: right"> 
Студент <b>420</b> группы 

__Трапезников Михаил Юрьевич__
</div>

## Описание алгоритма

Основные идеи параллелизации были описаны в отчете прошлого года, поэтому в данном файле указываются __только__ основные аспекты с краткими доводами:
1. Параллелизация:
   1. Алгоритм представляет из себя цикличное выполнение с 1 уровнем вложенности:
      1. __Внешний цикл__ - проход по основной диагонали матрицы с целью актуализации значения ранга матрицы;
      2. __Внутренний цикл__ - обновление последующих строк матрицы(лежащих ниже), приводя ее таким образом к ступенчатому виду.
   2. Параллельность программы заключается в:
      1. __Внешний цикл последователен__, т.к. иначе алгоритм будет некорректен(либо хаотичный доступ, либо последовательная организация работы);
      2. __Внутренний цикл парарллелен__ - каждая нить владеет своим набором строк и опрерирует __только__ над ними.
2. Данные:
   1. Для тестирования создается __матрица с 1 на главной диагонали и 1 в правом столбце__(_правильный ответ_ - __ранг равен кол-ву строк матрицы__);

## Модификация для отказоустойчивости алгоритма

В качестве сценария продолжения работы программы в случае сбоя использовался следующий: 
> a) продолжить работу программы только на “исправных” процессах;

Для симуляции отказоустойчивости системы выбирается процесс(нить), который будет уничтожен сразу после создания:
- В предлагаемом решении таким процессом служит процесс с __рангом 1(то есть 2-ой, т.к. нумерация с 0)__. 
- При уничтожении процесса возникает сообщение, что данный процесс уничтожен:
```bash
******************************
Process with rank 2 was killed
******************************
```

### ULFM

Для возможности установки контрольных точек для корректного продолжения работы программы в случае сбоя решено было использовать `ULFM-версию MPI`, поэтому данная часть задания реализована на языке `C`.

Для установления контрольных точек в программе использовался __собственный контроль ошибок__: 

```c
MPI_Errhandler errh;
MPI_Comm_create_errhandler(err_handler, &errh);
MPI_Comm_set_errhandler(main_comm, errh);
```

Для корректной работы была создана функция-__контрольная точка__ `err_handler`:

```c
static void
err_handler(MPI_Comm *pcomm, int *perr, ...) {
    int error = *perr;
    char error_msg[MPI_MAX_ERROR_STRING];
    int nf, len;
    MPI_Group group_f;

    MPI_Comm_size(main_comm, &nproc);
    MPIX_Comm_failure_ack(main_comm);
    MPIX_Comm_failure_get_acked(main_comm, &group_f);
    MPI_Group_size(group_f, &nf);
    MPI_Error_string(error, error_msg, &len);
    printf("\n@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@\nRank \
    %d / %d: Got error %s. %d found dead\
    \n@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@\n", \
    rk+1, nproc, error_msg, nf);
    
    MPIX_Comm_shrink(main_comm, &main_comm);
    MPI_Comm_rank(main_comm, &rk);

    for (int i=0; i<N; i++) {
        map[i] = i%(nproc-1);
    }
}
```

Как следует из определения сценария работы программы, опишем поведение предложенного контролья ошибок:
1. Изначально получаем код ошибки `error`, создаем вспомогательную переменную `error_msg`, храняющую полный код ошибки(__текстовый__), заводим переменные для хранения числа _погибших_ процессов `nf` и для хранения текстовой длины сообщения ошибки `len`; Также сохраним группу процессов(в нашем случае группа - __один процесс__) в специальную переменную `group_f`:
    ```c
    int error = *perr;
    char error_msg[MPI_MAX_ERROR_STRING];
    int nf, len;
    MPI_Group group_f;

    ...
    ```
2. Затем собираем информацию из __глобального коммуникатора__ `main_comm`:
   - Число процессов(изначально определенных) `nproc`:
    ```c
    MPI_Comm_size(main_comm, &nproc);
    ```
   - Идентифицируем область ошибки и определяем область, где возникла данная ошибка:
    ```c
    MPIX_Comm_failure_ack(main_comm);
    MPIX_Comm_failure_get_acked(main_comm, &group_f);
    ```
   - Определяем размер _проблемной_ группы `nf` и сопоставляем идентификатору ошибки `error` его текстовую интерпретацию `error_msg` длины `len`:
    ```c
    MPI_Group_size(group_f, &nf);
    MPI_Error_string(error, error_msg, &len);
    ```
   - Выводим служебную информацию от живых процессов об ошибке, указываем их номер, общее количество, описываем ошибку и указываем количество умерших процессов:
    ```c
    printf("\n@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@\nRank \
    %d / %d: Got error %s. %d found dead\
    \n@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@\n", \
    rk+1, nproc, error_msg, nf);
    ```
    Пример такого вывода выглядит так:
    ```bash
    @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@
    Rank 3 / 3: Got error MPI_ERR_PROC_FAILED: Process Failure. 1 found dead
    @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@
    ```
    - Обновляем общий коммуникатор, исключая из него умерший процесс и производим __переприсваивание рангов__ живым процессам(теперь их номера от 0 до 1 в примере для 3-х процессов):
    ```c
    MPIX_Comm_shrink(main_comm, &main_comm);
    MPI_Comm_rank(main_comm, &rk);
    ```
    - Далее __реализуется сценарий а)__:
      - Так как идея параллелизации программы - разделение принадлежности номеров строк разным процессам и запрет на изменение __не своих строк__, то достаточно переприсвоить номера строк на новые ранги и продолжить работу, т.к. все, что выполнилось до этого момента корректно, ибо внешний цикл последователен:
      ```c
      for (int i=0; i<N; i++) {
        map[i] = i%(nproc-1);
      }
      ```
    - Пример такого продолжения можно показать на примере отладочной информации принадлежности номеров строк процессам:
      - __До сбоя__:
      ```bash
      INIT MAP
        [MATRIX ROW] <-> [MPI RANK]
            0 <-> 0
            1 <-> 1
            2 <-> 2
            3 <-> 0
            4 <-> 1
            5 <-> 2
            6 <-> 0
            7 <-> 1
            8 <-> 2
            9 <-> 0
            10 <-> 1
            11 <-> 2
            12 <-> 0
            13 <-> 1
            14 <-> 2
      ```
      - И __после сбоя__:
      ```bash
      FINAL MAP
        [MATRIX ROW] <-> [MPI RANK]
            0 <-> 0
            1 <-> 1
            2 <-> 0
            3 <-> 1
            4 <-> 0
            5 <-> 1
            6 <-> 0
            7 <-> 1
            8 <-> 0
            9 <-> 1
            10 <-> 0
            11 <-> 1
            12 <-> 0
            13 <-> 1
            14 <-> 0
      ```

Оставшаяся часть алгоритма работает, как и прежде:
- Выводит время, полученное через инструкции `MPI`, изначальное число процессов и предсказанный ранг матрицы:
```bash
Time in seconds=0.0129482s
Threads num is 3
Rank is 15
```

### Общий код программы и правила ее запуска

#### Общий код программы

Код программы(язык `C`):
```c
#include <mpi.h>
#include <mpi-ext.h>
#include <math.h>
#include <stdlib.h>
#include <signal.h>
#include <stdio.h>
#include <sys/time.h>

int N;
float *A;
int *map;
int rk, nproc;
MPI_Comm main_comm;
#define A(i,j) A[(i)*(N+1)+(j)]

static void
err_handler(MPI_Comm *pcomm, int *perr, ...) {
    int error = *perr;
    char error_msg[MPI_MAX_ERROR_STRING];
    int nf, len;
    MPI_Group group_f;

    MPI_Comm_size(main_comm, &nproc);
    MPIX_Comm_failure_ack(main_comm);
    MPIX_Comm_failure_get_acked(main_comm, &group_f);
    MPI_Group_size(group_f, &nf);
    MPI_Error_string(error, error_msg, &len);
    printf("\n@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@\nRank \
    %d / %d: Got error %s. %d found dead\
    \n@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@\n", \
    rk+1, nproc, error_msg, nf);
    
    MPIX_Comm_shrink(main_comm, &main_comm);
    MPI_Comm_rank(main_comm, &rk);

    for (int i=0; i<N; i++) {
        map[i] = i%(nproc-1);
    }
}

int main(int argc,char **argv) {
    MPI_Init(&argc, &argv);
    double time0, time1;
    FILE *in;
    int i,j,k;
    int local_rk = 0, global_rk = 0;

    i=sscanf(argv[1],"%d", &N);
    if(i<1) {
        printf ("Wrong parameters . Run ./ test N"); 
        exit (2) ;
    }

    MPI_Status status ;
    map = malloc(sizeof(int)*N);
    MPI_Comm_rank(MPI_COMM_WORLD, &rk );
    MPI_Comm_size(MPI_COMM_WORLD, &nproc);
    main_comm = MPI_COMM_WORLD;

    MPI_Errhandler errh;
    MPI_Comm_create_errhandler(err_handler, &errh);
    MPI_Comm_set_errhandler(main_comm, errh);

    /* create arrays */
    A=(float *)malloc(N*(N+1)*sizeof(float));
    if (rk == 0) {
        printf ("GAUSS %dx%d\n==================================\n" ,N, N) ;
    }

    /* initialize array A*/
    for(i=0; i<=N-1; i++)
        for(j=0; j<=N; j++)
            if (i==j || j==N)
                A(i, j) = 1.f;
            else
                A(i, j)=0.f;

    /* elimination */
    MPI_Bcast (&A(0, 0), N*N, MPI_FLOAT, 0, main_comm);
    for (i=0; i<N; i++) {
        map[i] = i%nproc;
    }

    if (rk == 0) {
        printf("\nINIT MAP\n\t[MATRIX ROW] <-> [MPI RANK]\n");
        for (i=0; i< N; ++i) {
            printf("\t\t%d <-> %d\n", i, map[i]);
        }
        time0 = MPI_Wtime() ;
    }

    if (rk == 1) {
        printf("******************************\n");
        printf("Process with rank %d was killed\n", rk+1);
        printf("******************************\n");
        raise(SIGKILL);
    }

    for (i=0; i<N; i++) {
        checkpoint:
        MPI_Barrier(main_comm);
        if (map[i] == 2) {
            printf("INVALID ROOT!!!!\n");
        }
        MPI_Bcast (&A(i, i) , N-i , MPI_FLOAT, map[i] , main_comm);
        
        if (map[i] == rk) {
            if (A(i, i) != 0) {
                ++local_rk ;
            } else {
                break ;
            }
        }
        for(k=i+1; k<N; k++) {
            if(map[k] == rk) {
                for(j=i+1; j<=N; j++) {
                    A(k,j)=A(k,j)-A(k, i)*A(i,j)/A(i, i);
                }
            }
        }
    }

    MPI_Reduce(&local_rk , &global_rk , 1, MPI_FLOAT, MPI_SUM, 0, main_comm) ;

    /* reverse substitution */
    if (rk == 0) {
        time1 = MPI_Wtime();
        printf("\nFINAL MAP\n\t[MATRIX ROW] <-> [MPI RANK]\n");
        for (i=0; i< N; ++i) {
            printf("\t\t%d <-> %d\n", i, map[i]);
        }
        printf("==================================\n");
        printf("Time in seconds=%gs\n", time1 - time0); 
        printf("Threads num is %d\n", nproc); 
        printf("Rank is %d\n", global_rk);
    }

    MPI_Finalize();
    free(A);

    return 0;
}              
```

#### Правила компиляции и установки среды

Для использования `ULFM`-версии `MPI` использовался образ `Docker`'а. ОС, где разрабатывалось решение, была `Mac OS X`, поэтому для работы `Docker` потребовалась версия `Docker Desktop`:
1. Устанавливаем нужный образ `Docker`:
    ```bash
    docker pull abouteiller/mpi-ft-ulfm
    ```
2. Запускаем среду(__возможны отличия в связи с разными ОС!!!__):
    ```bash
    docker exec -it \
    2036c57fc2469160ac2fedc896c3babe0f0629858be61cbc21a9746a89c87e24 /bin/sh
    ```
3. На данном этапе в командной строке должна быть текущей директорией - `/sandbox`. Для удобства создаю директорию `gauss_ulfm`:
    ```bash
    mkdir gauss_ulfm && cd gauss_ulfm
    ```
4. Перемещаю в данную директорию файл с кодом `gauss_ulfm.c`, затем компилирую программу средствами `mpi`:
    ```bash
    mpicc -o gauss_ulfm gauss_ulfm.c
    ```
5. Затем получается исполняемый файл в этой же директории `/sandbox/gauss_ulfm` `gauss_ulfm`. Для его запуска выполняю команду:
    ```bash
    mpirun -n [NPROC] --map-by :OVERSUBSCRIBE --with-ft ulfm ./test [MATRIX SIZE]
    ```
    где:
    -  `NPROC` - желаемое число процессов; 
    -  параметр `--map-by :OVERSUBSCRIBE` дает возможность создавать процессов больше, чем имеется физических ядер; 
    -  параметр `--with-ft ulfm` указывает, что используется версия с `ulfm`;
    -  `MATRIX SIZE` задает размер матрицы(он же будет правильным ответом).

__Пример работы__:
- Для тестового запуска использовалось __3__ процесса(что меньше количества физических ядер) и размер матрицы __15__:
  ```bash
  /sandbox/gauss_ulfm $ mpirun \
  -n 3 \
  --map-by :OVERSUBSCRIBE \
  --with-ft ulfm \
  ./gauss_ulfm 15
  
  GAUSS 15x15
    ==================================
    ******************************
    Process with rank 2 was killed
    ******************************

    INIT MAP
        [MATRIX ROW] <-> [MPI RANK]
            0 <-> 0
            1 <-> 1
            2 <-> 2
            3 <-> 0
            4 <-> 1
            5 <-> 2
            6 <-> 0
            7 <-> 1
            8 <-> 2
            9 <-> 0
            10 <-> 1
            11 <-> 2
            12 <-> 0
            13 <-> 1
            14 <-> 2

    @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@
    Rank 3 / 3: Got error MPI_ERR_PROC_FAILED: Process Failure. 1 found dead
    @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@

    @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@
    Rank 1 / 3: Got error MPI_ERR_PROC_FAILED: Process Failure. 1 found dead
    @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@

    FINAL MAP
        [MATRIX ROW] <-> [MPI RANK]
            0 <-> 0
            1 <-> 1
            2 <-> 0
            3 <-> 1
            4 <-> 0
            5 <-> 1
            6 <-> 0
            7 <-> 1
            8 <-> 0
            9 <-> 1
            10 <-> 0
            11 <-> 1
            12 <-> 0
            13 <-> 1
            14 <-> 0
    ==================================
    Time in seconds=0.0129482s
    Threads num is 3
    Rank is 15
  ```