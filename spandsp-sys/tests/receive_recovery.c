/* Native fault injection complements the real PCMA/PCMU/T.38 exchange tests. */
#define SPANDSP_EXPOSE_INTERNAL_STRUCTURES
#include "spandsp.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>

#define WIDTH 1728
#define ROWS 400
static unsigned char source_pixel(int row, int x) { return (unsigned char)(((unsigned)(row*1728+x)*2654435761u) >> 24); }
static int source(void *user, uint8_t *buf, size_t len) {
    int *row = user;
    if (*row == ROWS) return 0;
    for (int x=0; x<(int)len; x++) buf[x]=source_pixel(*row,x);
    (*row)++;
    return len;
}
static t30_state_t *receiver(const char *path, int compression, int preserve) {
    t30_state_t *s=t30_init(NULL,false,NULL,NULL,NULL,NULL,NULL,NULL);
    assert(s);
    assert(t30_set_receive_recovery(s,preserve)==0);
    assert(t4_rx_init(&s->t4.rx,path,T4_COMPRESSION_T6));
    s->operation_in_progress=1; /* OPERATION_IN_PROGRESS_T4_RX in t30.c */
    s->t4.rx.recovery=&s->recovery;
    s->recovery.closed=0;
    t4_rx_set_image_width(&s->t4.rx,WIDTH);
    t4_rx_set_rx_encoding(&s->t4.rx,compression);
    t4_rx_start_page(&s->t4.rx);
    for(int i=0;i<256;i++) s->ecm_len[i]=-1;
    s->ecm_frames=-1;
    return s;
}
static void check_image(const char *path,t30_state_t *s,int expected_pages) {
    assert(t30_receive_output_error(s)==0);
    assert(t30_receive_output_closed(s));
    assert(t30_receive_page_count(s)==expected_pages);
    if (!expected_pages) {assert(access(path,F_OK)!=0); return;}
    TIFF *t=TIFFOpen(path,"r");assert(t);
    for(int i=0;i<expected_pages;i++) {
        t4_rx_recovery_page_t p;assert(t30_receive_page(s,i,&p)==0);
        uint32_t w,h;assert(TIFFGetField(t,TIFFTAG_IMAGEWIDTH,&w));assert(TIFFGetField(t,TIFFTAG_IMAGELENGTH,&h));
        assert(w==WIDTH && h==(uint32_t)p.rows && h>0 && h<=ROWS);
        for(uint32_t r=0;r<h;r++) {
            uint8_t row[WIDTH/8];assert(TIFFReadScanline(t,row,r,0)==1);
            for(int x=0;x<WIDTH/8;x++) assert(row[x]==source_pixel(r,x));
        }
        assert(!!TIFFReadDirectory(t)==(i+1<expected_pages));
    }
    TIFFClose(t);
}
struct file_io {int fd, fail_write, fail_close, close_calls;};
static tmsize_t io_read(thandle_t h,void *b,tmsize_t n){return read(((struct file_io*)h)->fd,b,n);}
static tmsize_t io_write(thandle_t h,void *b,tmsize_t n){struct file_io *f=h;return f->fail_write ? -1 : write(f->fd,b,n);}
static toff_t io_seek(thandle_t h,toff_t o,int w){return lseek(((struct file_io*)h)->fd,o,w);}
static int io_close(thandle_t h){struct file_io *f=h;f->close_calls++;int r=close(f->fd);return f->fail_close?-1:r;}
static toff_t io_size(thandle_t h){struct file_io *f=h;off_t p=lseek(f->fd,0,SEEK_CUR),n=lseek(f->fd,0,SEEK_END);lseek(f->fd,p,SEEK_SET);return n;}
static void output_errors(const char *path,const uint8_t *data,int len) {
    for(int close_error=0;close_error<2;close_error++) {
        t30_state_t *s=receiver(path,T4_COMPRESSION_T4_1D,1);
        TIFFClose(s->t4.rx.tiff.tiff_file);
        struct file_io io={open(path,O_CREAT|O_TRUNC|O_RDWR,0600),0,close_error,0};assert(io.fd>=0);
        s->t4.rx.tiff.tiff_file=TIFFClientOpen(path,"w",&io,io_read,io_write,io_seek,io_close,io_size,NULL,NULL);
        assert(s->t4.rx.tiff.tiff_file);
        t4_rx_put(&s->t4.rx,data,len/2);
        io.fail_write=!close_error;
        t30_receive_finalize(s);
        assert(t30_receive_output_error(s)&(close_error?4:1));
        assert(io.close_calls==1);
        if(close_error) assert(!t30_receive_output_closed(s));
        t30_receive_finalize(s);assert(io.close_calls==1);
        t30_free(s);assert(io.close_calls==1);
        unlink(path);
    }
}
int main(int argc,char **argv) {
    assert(argc==2);const char *path=argv[1];
    uint8_t *data=malloc(1024*1024);assert(data);
    for(int c=0;c<4;c++) {
        int compression=(int[]){T4_COMPRESSION_T4_1D,T4_COMPRESSION_T4_2D,T4_COMPRESSION_T6,T4_COMPRESSION_T85}[c];
        int row=0, len;
        if(c==3) {
            t85_encode_state_t *enc=t85_encode_init(NULL,WIDTH,ROWS,source,&row);assert(enc);
            len=t85_encode_get(enc,data,1024*1024);t85_encode_free(enc);
        } else {
            t4_t6_encode_state_t *enc=t4_t6_encode_init(NULL,compression,WIDTH,ROWS,source,&row);assert(enc);
            len=t4_t6_encode_get(enc,data,1024*1024);t4_t6_encode_free(enc);
        }
        assert(len>2048);
        for(int cut=0;cut<20;cut++) {
            int n=cut==0?0:(len*cut)/20;
            t30_state_t *s=receiver(path,compression,1);
            if(n) t4_rx_put(&s->t4.rx,data,n);
            t30_stats_t before,after;t30_get_transfer_statistics(s,&before);
            int rows=s->t4.rx.decoded_rows;
            t30_receive_finalize(s);t30_get_transfer_statistics(s,&after);
            assert(!memcmp(&before,&after,sizeof(before)));
            assert(!t30_receive_completed(s));assert(after.pages_rx==0);
            check_image(path,s,rows>0);
            t30_receive_finalize(s);check_image(path,s,rows>0);
            t30_free(s);unlink(path);
        }
        /* Uncommitted ECM: recover only contiguous full CRC-accepted frames.
           A later valid frame must never be concatenated across a missing one. */
        for(int gap=0;gap<8;gap++) {
            t30_state_t *s=receiver(path,compression,1);s->error_correcting_mode=1;s->octets_per_ecm_frame=256;
            for(int i=0;i<8;i++) {s->ecm_len[i]=256;memcpy(s->ecm_data[i],data+256*i,256);}
            s->ecm_len[gap]=-1;
            t30_receive_finalize(s);assert(s->rx_page_number==0);
            int rows=s->t4.rx.decoded_rows;
            if(gap==0) assert(rows==0);
            if(gap==7) assert(rows>0);
            check_image(path,s,rows>0);
            t30_free(s);unlink(path);
        }
        /* A complete page survives even if no row of the next page decodes. */
        t30_state_t *s=receiver(path,compression,1);
        t4_rx_put(&s->t4.rx,data,len);assert(t4_rx_end_page(&s->t4.rx)==0);s->rx_page_number++;
        t4_rx_start_page(&s->t4.rx);t4_rx_put(&s->t4.rx,data,1);
        t30_set_status(s,T30_ERR_RX_DCNPHD);
        t30_terminate(s);assert(t30_receive_completed(s));
        t30_receive_finalize(s);t30_stats_t stats;t30_get_transfer_statistics(s,&stats);
        assert(stats.pages_rx==1 && stats.current_status==T30_ERR_RX_DCNPHD);
        check_image(path,s,1);t30_free(s);unlink(path);
        /* Corrupt scanline after a good prefix. Retain no concealed rows. */
        if(c==0) {
            s=receiver(path,compression,1);
            t4_rx_put(&s->t4.rx,data,len/4);
            uint8_t bad[128]={0};t4_rx_put(&s->t4.rx,bad,sizeof(bad));
            t4_rx_put(&s->t4.rx,data+len/2,len/4);
            assert(s->t4.rx.decoder.t4_t6.bad_rows>0);
            t30_receive_finalize(s);
            assert(t30_receive_missing_tail(s));check_image(path,s,1);
            t4_rx_recovery_page_t p;t30_receive_page(s,0,&p);assert(p.bad_rows>0 && p.partial);
            t30_free(s);unlink(path);
            output_errors(path,data,len);
        }
    }
    free(data);puts("native recovery fault tests passed");return 0;
}
