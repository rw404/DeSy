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
    printf("\n@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@\nRank %d / %d: Got error %s. %d found dead\n@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@\n", rk+1, nproc, error_msg, nf);
    
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