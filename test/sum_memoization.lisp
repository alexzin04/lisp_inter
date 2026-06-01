(define (sum-unique n)
  (if (< n 1)
      0
      (+ n (sum-unique (- n 1)))))

(sum-unique 500)