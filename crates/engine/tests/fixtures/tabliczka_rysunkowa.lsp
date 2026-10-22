;;; ==========================================================================
;;; CoreCAD AutoLISP - Tabliczka Rysunkowa (metryczna PN/ISO)
;;; Plik: demo/lisp/tabliczka_rysunkowa.lsp
;;;
;;; Polecenia:
;;;   TABLICZKA - wstawia znormalizowana tabliczke rysunkowa w zadanym punkcie
;;;
;;; Funkcje programowe:
;;;   (wstaw-linie p1 p2 layer)
;;;   (wstaw-napis pt txt h layer bold)
;;;   (wstaw-tabliczke pt tytul autor skala data arkusz)
;;; ==========================================================================

(defun wstaw-linie (p1 p2 layer)
  (if (null layer) (setq layer "TABLICZKA"))
  (entmake (list
    '(0 . "LINE")
    (cons 8 layer)
    (cons 10 p1)
    (cons 11 p2)
  ))
)

(defun wstaw-napis (pt txt h layer / dxf)
  (if (null layer) (setq layer "TABLICZKA"))
  (if (null h) (setq h 2.5))
  (setq dxf (list
    '(0 . "TEXT")
    (cons 8 layer)
    (cons 10 pt)
    (cons 11 pt)
    (cons 40 h)
    (cons 1 txt)
    (cons 50 0.0)
    (cons 72 0) ; Left
    (cons 73 0) ; Baseline
  ))
  (entmake dxf)
)

;; Wstawia tabliczke metryczna (szerokosc 185 mm, wysokosc 50 mm)
;; pt - lewy dolny naroznik tabliczki
(defun wstaw-tabliczke (pt tytul autor skala data arkusz / sc w h x0 y0 layer)
  (if (null tytul) (setq tytul "PROJEKT ARCHITEKTONICZNO-BUDOWLANY"))
  (if (null autor) (setq autor "MKM Labs - CoreCAD"))
  (if (null skala) (setq skala "1:100"))
  (if (null data) (setq data "2026-10"))
  (if (null arkusz) (setq arkusz "A-01"))

  ;; Skalowanie: jesli INSUNITS = 6 (metry), wymiary w metrach (0.185 x 0.050)
  (setq sc (if (= (getvar "INSUNITS") 6) 0.001 1.0))
  (setq w (* 185.0 sc))
  (setq h (* 50.0 sc))
  (setq x0 (car pt))
  (setq y0 (cadr pt))
  (setq layer "TABLICZKA")

  ;; Ramka zewnetrzna
  (wstaw-linie (list x0 y0) (list (+ x0 w) y0) layer)
  (wstaw-linie (list (+ x0 w) y0) (list (+ x0 w) (+ y0 h)) layer)
  (wstaw-linie (list (+ x0 w) (+ y0 h)) (list x0 (+ y0 h)) layer)
  (wstaw-linie (list x0 (+ y0 h)) (list x0 y0) layer)

  ;; Podzial poziomy (linie wewnetrzne)
  (wstaw-linie (list x0 (+ y0 (* 15.0 sc))) (list (+ x0 w) (+ y0 (* 15.0 sc))) layer)
  (wstaw-linie (list x0 (+ y0 (* 30.0 sc))) (list (+ x0 w) (+ y0 (* 30.0 sc))) layer)
  (wstaw-linie (list x0 (+ y0 (* 40.0 sc))) (list (+ x0 w) (+ y0 (* 40.0 sc))) layer)

  ;; Podzial pionowy
  (wstaw-linie (list (+ x0 (* 120.0 sc)) y0) (list (+ x0 (* 120.0 sc)) (+ y0 (* 30.0 sc))) layer)
  (wstaw-linie (list (+ x0 (* 155.0 sc)) y0) (list (+ x0 (* 155.0 sc)) (+ y0 (* 15.0 sc))) layer)

  ;; Etykiety i teksty (wysokosci: naglowek 3.5mm, drobne 2.0mm)
  (wstaw-napis (list (+ x0 (* 3.0 sc)) (+ y0 (* 43.0 sc))) "INWESTYCJA / OBIEKT:" (* 2.0 sc) layer)
  (wstaw-napis (list (+ x0 (* 5.0 sc)) (+ y0 (* 33.0 sc))) tytul (* 3.5 sc) layer)

  (wstaw-napis (list (+ x0 (* 3.0 sc)) (+ y0 (* 24.0 sc))) "PROJEKTANT / AUTOR:" (* 2.0 sc) layer)
  (wstaw-napis (list (+ x0 (* 5.0 sc)) (+ y0 (* 18.0 sc))) autor (* 3.0 sc) layer)

  (wstaw-napis (list (+ x0 (* 3.0 sc)) (+ y0 (* 9.0 sc))) "FAZA:" (* 2.0 sc) layer)
  (wstaw-napis (list (+ x0 (* 5.0 sc)) (+ y0 (* 3.0 sc))) "PROJEKT WYKONAWCZY" (* 2.5 sc) layer)

  (wstaw-napis (list (+ x0 (* 123.0 sc)) (+ y0 (* 24.0 sc))) "SKALA:" (* 2.0 sc) layer)
  (wstaw-napis (list (+ x0 (* 130.0 sc)) (+ y0 (* 18.0 sc))) skala (* 3.5 sc) layer)

  (wstaw-napis (list (+ x0 (* 123.0 sc)) (+ y0 (* 9.0 sc))) "DATA:" (* 2.0 sc) layer)
  (wstaw-napis (list (+ x0 (* 125.0 sc)) (+ y0 (* 3.0 sc))) data (* 2.5 sc) layer)

  (wstaw-napis (list (+ x0 (* 158.0 sc)) (+ y0 (* 9.0 sc))) "ARKUSZ:" (* 2.0 sc) layer)
  (wstaw-napis (list (+ x0 (* 160.0 sc)) (+ y0 (* 3.0 sc))) arkusz (* 3.0 sc) layer)

  (princ (strcat "\nWstawiono tabliczke rysunkowa w punkcie (" (rtos x0 2 1) ", " (rtos y0 2 1) ")."))
  T
)

;; Polecenie interaktywne
(defun c:tabliczka (/ pt tytul autor skala data arkusz)
  (setq pt (getpoint "\nWskaz lewy dolny naroznik tabliczki rysunkowej: "))
  (if (null pt)
    (princ "\nAnulowano wstawianie tabliczki.")
    (progn
      (setq tytul (getstring T "\nTytul rysunku <PROJEKT BUDOWLANY>: "))
      (if (or (null tytul) (= tytul "")) (setq tytul "PROJEKT BUDOWLANY"))

      (setq autor (getstring T "\nProjektant <MKM Labs>: "))
      (if (or (null autor) (= autor "")) (setq autor "MKM Labs"))

      (setq skala (getstring "\nSkala <1:100>: "))
      (if (or (null skala) (= skala "")) (setq skala "1:100"))

      (wstaw-tabliczke pt tytul autor skala "2026-10" "A-01")
    )
  )
  (princ)
)

(princ "\nWczytano modul tabliczki rysunkowej. Uruchom: TABLICZKA")
(princ)
