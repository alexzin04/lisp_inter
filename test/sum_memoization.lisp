(define (sum-unique n acc)
  (if (< n 1)
      acc
      (sum-unique (- n 1) (+ acc n))))

(sum-unique 50000 0)
