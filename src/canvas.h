#pragma once
#include "document.h"
#include <QWidget>
#include <QRectF>
namespace studio {
class Canvas : public QWidget {
    Q_OBJECT
public:
    explicit Canvas(QWidget* parent=nullptr);
    void setDocument(const Document* doc);
    void select(int node);
    void setZoom(double zoom);
signals:
    void selected(int node);
    void geometryEdited(int node, int x, int y, int width, int height, bool resize);
protected:
    void paintEvent(QPaintEvent*) override;
    void mousePressEvent(QMouseEvent*) override;
    void mouseMoveEvent(QMouseEvent*) override;
    void mouseReleaseEvent(QMouseEvent*) override;
    void wheelEvent(QWheelEvent*) override;
private:
    const Document* doc_=nullptr;
    QVector<QRectF> boxes_;
    int selection_=-1, dragging_=-1;
    double zoom_=1.0;
    bool resizing_=false;
    QPointF dragStart_;
    QRectF dragRect_;
    void layout();
    QPointF origin() const;
};
}
