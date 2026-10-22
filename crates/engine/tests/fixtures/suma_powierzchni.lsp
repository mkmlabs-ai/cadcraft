;;; ==========================================================================
;;; CoreCAD AutoLISP - Suma Powierzchni Zamknietych Polilinii w m²
;;; Plik: demo/lisp/suma_powierzchni.lsp
;;;
;;; Polecenia:
;;;   SUMAPOW       - liczy i sumuje powierzchnie wybranych zamknietych polilinii w m²
;;;   POWIERZCHNIE  - alias do SUMAPOW
;;;
;;; Funkcje programowe:
;;;   (policz-pole-wieloboku pts)
;;;   (pobierz-wierzcholki-ent ent-data)
;;;   (suma-powierzchni-ss ss)
;;; ==========================================================================

;; Oblicza pole wieloboku 2D ze wspolrzednych wierzcholkow (wzor Gaussa / Shoelace)
(defun policz-pole-wieloboku (pts / n i sum p1 p2)
  (setq n (length pts))
  (if (< n 3)
    0.0
    (progn
      (setq sum 0.0)
      (setq i 0)
      (while (< i n)
        (setq p1 (nth i pts))
        (setq p2 (nth (rem (1+ i) n) pts))
        (setq sum (+ sum (- (* (car p1) (cadr p2)) (* (car p2) (cadr p1)))))
        (setq i (1+ i))
      )
      (* 0.5 (abs sum))
    )
  )
)

;; Wyciaga liste punktow (x y) z definicji DXF polilinii (grupy 10)
(defun pobierz-wierzcholki-ent (dxf-data / res item)
  (setq res nil)
  (foreach item dxf-data
    (if (= (car item) 10)
      (setq res (cons (list (cadr item) (caddr item)) res))
    )
  )
  (reverse res)
)

;; Sprawdza czy polilinia jest zamknieta (flaga 70 bit 1)
(defun czy-zamknieta (dxf-data / flag)
  (setq flag (cdr (assoc 70 dxf-data)))
  (if flag
    (= (logand flag 1) 1)
    nil
  )
)

;; Glowna funkcja sumujaca dla danego selection setu
(defun suma-powierzchni-ss (ss / i len ent dxf pts pole pole-m2 suma-m2 scale count)
  (if (null ss)
    0.0
    (progn
      ;; Skala jednostek do m²:
      ;; INSUNITS: 4 = mm -> 1e-6, 6 = m -> 1.0, domyslnie jesli wymiary > 500 to mm
      (setq scale (if (= (getvar "INSUNITS") 6) 1.0 0.000001))
      (setq suma-m2 0.0)
      (setq count 0)
      (setq len (sslength ss))
      (setq i 0)
      (while (< i len)
        (setq ent (ssname ss i))
        (setq dxf (entget ent))
        (if (or (czy-zamknieta dxf) T) ; liczymy kazda zamknieta lub wielobok
          (progn
            (setq pts (pobierz-wierzcholki-ent dxf))
            (if (>= (length pts) 3)
              (progn
                (setq pole (policz-pole-wieloboku pts))
                (setq pole-m2 (* pole scale))
                (setq suma-m2 (+ suma-m2 pole-m2))
                (setq count (1+ count))
                (princ (strcat "\nElement " (itoa count) ": " (rtos pole-m2 2 2) " m²"))
              )
            )
          )
        )
        (setq i (1+ i))
      )
      (princ (strcat "\n------------------------------------"))
      (princ (strcat "\nLaczna powierzchnia (" (itoa count) " pom.): " (rtos suma-m2 2 2) " m²"))
      suma-m2
    )
  )
)

;; Polecenie uzytkownika
(defun c:sumapow (/ ss suma pt h txt)
  (princ "\nWybierz zamkniete polilinie do zsumowania (Enter = wszystkie w rysunku): ")
  (setq ss (ssget '((0 . "LWPOLYLINE"))))
  (if (null ss)
    (progn
      (princ "\nBrak zaznaczenia - pobieram wszystkie polilinie z rysunku...")
      (setq ss (ssget "X" '((0 . "LWPOLYLINE"))))
    )
  )

  (if (or (null ss) (= (sslength ss) 0))
    (princ "\nNie znaleziono zadnych polilinii.")
    (progn
      (setq suma (suma-powierzchni-ss ss))
      (setq pt (getpoint "\nWskaz punkt do wstawienia opisu sumy (lub Enter aby pominac): "))
      (if pt
        (progn
          (setq h (if (= (getvar "INSUNITS") 6) 0.35 35.0))
          (setq txt (strcat "Suma pow.: " (rtos suma 2 2) " m%%d"))
          (entmake (list
            '(0 . "TEXT")
            '(8 . "OPISY")
            (cons 10 pt)
            (cons 11 pt)
            (cons 40 h)
            (cons 1 (strcat "Powierzchnia: " (rtos suma 2 2) " m2"))
            (cons 50 0.0)
            (cons 72 0)
            (cons 73 0)
          ))
          (princ "\nWstawiono etykiete z suma powierzchni.")
        )
      )
    )
  )
  (princ)
)

(defun c:powierzchnie () (c:sumapow))

(princ "\nWczytano modul sumowania powierzchni. Uruchom: SUMAPOW")
(princ)
